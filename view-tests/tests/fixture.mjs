// The Playwright fixture of the page tests: per test, `target/debug/surf-view-fixture` (a page
// server, surfd in process, its own XDG_RUNTIME_DIR under the machine's), its commands, and a
// record of the page: every request (its status; its headers and body when a test reads them),
// every console message, page error, dialog and securitypolicyviolation. At the end of the test
// the fixture's standard input is closed and the test waits for it to exit.

import { test as base, expect } from "@playwright/test";
import { spawn } from "node:child_process";
import path from "node:path";
import readline from "node:readline";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const PROGRAM = path.join(ROOT, "target", "debug", "surf-view-fixture");
const START_WAIT = 15_000;
const EXIT_WAIT = 15_000;

// One running surf-view-fixture.
export class Fixture {
  static async start(args = []) {
    const child = spawn(PROGRAM, args, { stdio: ["pipe", "pipe", "inherit"] });
    const fixture = new Fixture(child);
    const line = await fixture.next(START_WAIT, "the fixture's first line");
    const first = JSON.parse(line);
    fixture.view = first.view;
    fixture.piped = first.piped;
    fixture.pages = first.pages;
    fixture.unreachable = first.unreachable;
    const view = new URL(first.view);
    fixture.origin = view.origin;
    fixture.token = view.hash.slice(1);
    return fixture;
  }

  constructor(child) {
    this.child = child;
    this.lines = [];
    this.waiting = [];
    this.chain = Promise.resolve();
    this.exited = new Promise((resolve) => child.on("exit", (code, signal) => resolve({ code, signal })));
    readline.createInterface({ input: child.stdout }).on("line", (line) => {
      const waiter = this.waiting.shift();
      if (waiter) {
        waiter(line);
      } else {
        this.lines.push(line);
      }
    });
  }

  next(within, what) {
    if (this.lines.length > 0) {
      return Promise.resolve(this.lines.shift());
    }
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("no answer within " + within + " ms: " + what)), within);
      this.waiting.push((line) => {
        clearTimeout(timer);
        resolve(line);
      });
    });
  }

  // One command, its words joined by a tab; its JSON answer. Commands run one after another.
  command(...words) {
    const run = async () => {
      this.child.stdin.write(words.join("\t") + "\n");
      return JSON.parse(await this.next(60_000, words[0]));
    };
    const result = this.chain.then(run, run);
    this.chain = result.catch(() => {});
    return result;
  }

  async open(url) {
    const answer = url === undefined ? await this.command("open") : await this.command("open", url);
    if (!answer.id) {
      throw new Error("open answered " + JSON.stringify(answer));
    }
    return answer.id;
  }

  async agent(id, ...words) {
    const answer = await this.command("agent", id, ...words);
    if (answer.status === undefined) {
      throw new Error("agent answered " + JSON.stringify(answer));
    }
    return answer.status;
  }

  // `surf-agent <words>` as session `id` with `input` on its standard input (fill <handle> -),
  // so that the value is in no command line.
  async agentStdin(id, input, ...words) {
    const answer = await this.command("agent-stdin", id, input, ...words);
    if (answer.status === undefined) {
      throw new Error("agent-stdin answered " + JSON.stringify(answer));
    }
    return answer.status;
  }

  kill(id) {
    return this.command("kill", id);
  }

  stopSurfd() {
    return this.command("stop-surfd");
  }

  // Takes every connection through `piped` away, and refuses new ones until mend().
  cut() {
    return this.command("cut");
  }

  mend() {
    return this.command("mend");
  }

  // GET of a route of the view with the view token, from Node: the answer's text.
  async apiText(route) {
    const answer = await fetch(this.origin + route, {
      headers: { Authorization: "Bearer " + this.token },
    });
    if (answer.status !== 200) {
      throw new Error(route + " answered " + answer.status);
    }
    return answer.text();
  }

  async api(route) {
    return JSON.parse(await this.apiText(route));
  }

  async close() {
    if (this.child.exitCode === null && this.child.signalCode === null) {
      this.child.stdin.end();
    }
    const timer = new Promise((resolve) => setTimeout(() => resolve(null), EXIT_WAIT));
    const exited = await Promise.race([this.exited, timer]);
    if (exited === null) {
      this.child.kill("SIGKILL");
      throw new Error("the fixture did not exit within " + EXIT_WAIT + " ms of its input's end");
    }
    if (exited.code !== 0) {
      throw new Error("the fixture exited with " + JSON.stringify(exited));
    }
  }
}

// What the page did.
export class Record {
  constructor() {
    this.requests = [];
    this.console = [];
    this.errors = [];
    this.dialogs = [];
    this.violations = [];
    // The main frame's address after each navigation, same-document ones included.
    this.addresses = [];
  }

  // The recorded requests whose path is `pathname` (the query and fragment aside).
  requestsOf(pathname) {
    return this.requests.filter((request) => request.path === pathname);
  }
}

export const test = base.extend({
  fixtureArgs: [[], { option: true }],

  record: async ({ page }, use) => {
    const record = new Record();
    let order = 0;
    page.on("request", (request) => {
      const url = new URL(request.url());
      const response = request.response().catch(() => null);
      let headers = null;
      let body = null;
      const entry = {
        order: order++,
        url: request.url(),
        path: url.pathname,
        method: request.method(),
        // Asked of the browser only when a test reads it (story:page-step-holds-under-load: a
        // request's headers and body asked for every request cost the workers and the browser
        // CPU seconds that no test used); once asked, kept.
        get headers() {
          headers ??= request.allHeaders().catch(() => request.headers());
          return headers;
        },
        status: response.then((answer) => (answer ? answer.status() : null)),
        // The body; never settles for a stream that stays open.
        get body() {
          body ??= response.then((answer) => (answer ? answer.text().catch(() => null) : null));
          return body;
        },
      };
      record.requests.push(entry);
    });
    page.on("framenavigated", (frame) => {
      if (frame === page.mainFrame()) {
        record.addresses.push(frame.url());
      }
    });
    page.on("console", (message) => record.console.push({ type: message.type(), text: message.text() }));
    page.on("pageerror", (error) => record.errors.push(String(error)));
    page.on("dialog", (dialog) => {
      record.dialogs.push(dialog.message());
      dialog.dismiss().catch(() => {});
    });
    await page.exposeBinding("surfTestViolation", (_source, violation) => record.violations.push(violation));
    await page.addInitScript(() => {
      document.addEventListener("securitypolicyviolation", (event) => {
        window.surfTestViolation({
          directive: event.violatedDirective,
          blocked: event.blockedURI,
          sample: event.sample,
        });
      });
    });
    await use(record);
  },

  // Starts further fixtures in the same test (case 17); each is closed at the end.
  startFixture: async ({}, use) => {
    const started = [];
    await use(async (args) => {
      const fixture = await Fixture.start(args);
      started.push(fixture);
      return fixture;
    });
    for (const fixture of started) {
      await fixture.close();
    }
  },

  surf: async ({ fixtureArgs }, use) => {
    const fixture = await Fixture.start(fixtureArgs);
    try {
      await use(fixture);
    } finally {
      await fixture.close();
    }
  },
});

export { expect };
