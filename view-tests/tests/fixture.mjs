// The Playwright fixture of the page tests: per test, `target/debug/surf-view-fixture` (a page
// server, surfd in process, its own XDG_RUNTIME_DIR under the machine's), its commands, and a
// record of the page: every request (its status; its headers and body when a test reads them),
// every console message, page error, dialog and securitypolicyviolation. At the end of the test
// the fixture's standard input is closed and the test waits for it to exit. Also the one wait
// for the live marker to read `live`, expectLive().

import { test as base, expect } from "@playwright/test";
import { spawn } from "node:child_process";
import path from "node:path";
import readline from "node:readline";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const PROGRAM = path.join(ROOT, "target", "debug", "surf-view-fixture");
const START_WAIT = 15_000;
const EXIT_WAIT = 15_000;
// How long each read after a failed wait for `live` may take.
const READ_WAIT = 2_000;
// How many of the page's latest fetch calls its note keeps.
const FETCHES_KEPT = 64;

// An init script of every page (the `page` fixture below): the page's own note of each fetch it
// makes, its path, when it was called and when its answer's head arrived or it failed, in ms
// from the navigation (performance.now()), in `surfTestFetches`. It makes no request and sends
// nothing to the test; it is read only after a failed wait for `live`.
function noteFetches(kept) {
  const calls = [];
  const original = window.fetch;
  Object.defineProperty(window, "surfTestFetches", { value: calls });
  window.fetch = function fetch(input, init) {
    const answer = Reflect.apply(original, window, [input, init]);
    let where;
    try {
      where = new URL(input instanceof Request ? input.url : String(input), location.href).pathname;
    } catch {
      where = String(input);
    }
    const call = { path: where, called: Math.round(performance.now()), head: null, status: null, failed: null, error: null };
    calls.push(call);
    if (calls.length > kept) {
      calls.shift();
    }
    answer.then(
      (response) => {
        call.head = Math.round(performance.now());
        call.status = response.status;
      },
      (error) => {
        call.failed = Math.round(performance.now());
        call.error = String(error && error.message ? error.message : error);
      },
    );
    return answer;
  };
}

// `promise`, or `fallback` when it has not settled within READ_WAIT.
async function within(promise, fallback) {
  let timer;
  const late = new Promise((resolve) => {
    timer = setTimeout(() => resolve(fallback), READ_WAIT);
  });
  try {
    return await Promise.race([promise, late]);
  } finally {
    clearTimeout(timer);
  }
}

const NOT_READ = `(not read within ${READ_WAIT} ms)`;

function shownText(value) {
  return value === "" || value === null ? "(empty)" : value;
}

function notRead(failure) {
  return `(not read: ${String(failure && failure.message ? failure.message : failure).split("\n")[0]})`;
}

// One fetch of the page's note: `<path> called at N ms, head at M ms (status S)`, or its head
// not arrived, or failed.
function fetchLine(call) {
  const where = call.path.replace(/[0-9a-f-]{36}$/, "<id>");
  if (call.head !== null) {
    return `${where} called at ${call.called} ms, head at ${call.head} ms (${call.status})`;
  }
  if (call.failed !== null) {
    return `${where} called at ${call.called} ms, failed at ${call.failed} ms (${call.error})`;
  }
  return `${where} called at ${call.called} ms, head not arrived`;
}

// What the page shows and did when `live` was not reached, each read once, all at the same
// time, each within READ_WAIT: the live marker's word and class, #message, the segments drawn;
// the page's /events fetches (sent, its head arrived or not, ms from the navigation) and its
// other fetches; the browser's own requests for /events and their answers; and whether the
// view answers `/` from here, outside the browser.
async function shows(page) {
  const origin = (() => {
    try {
      return new URL(page.url()).origin;
    } catch {
      return null;
    }
  })();
  const askView = async () => {
    if (origin === null || origin === "null") {
      return "(no origin)";
    }
    const started = Date.now();
    try {
      const answer = await fetch(origin + "/", { signal: AbortSignal.timeout(READ_WAIT) });
      await answer.arrayBuffer();
      return `${answer.status} in ${Date.now() - started} ms`;
    } catch (failure) {
      return `${notRead(failure)} after ${Date.now() - started} ms`;
    }
  };
  const [word, kind, message, drawn, fetches, requests, view] = await Promise.all([
    page.locator("#live-word").textContent({ timeout: READ_WAIT }).then(shownText, notRead),
    page.locator("#live").getAttribute("class", { timeout: READ_WAIT }).then(shownText, notRead),
    page.locator("#message").textContent({ timeout: READ_WAIT }).then(shownText, notRead),
    within(page.locator("#actions > li.segment").count(), NOT_READ).catch(notRead),
    within(page.evaluate(() => (Array.isArray(window.surfTestFetches) ? window.surfTestFetches : null)), NOT_READ).catch(notRead),
    within(page.requests(), NOT_READ).catch(notRead),
    askView(),
  ]);
  let stream;
  let others;
  if (Array.isArray(fetches)) {
    const events = fetches.filter((call) => call.path === "/events");
    stream =
      events.length === 0
        ? `not sent (the page made ${fetches.length} other fetch(es))`
        : events
            .map((call) =>
              call.head !== null
                ? `sent at ${call.called} ms from the navigation, head arrived at ${call.head} ms (${call.status})`
                : call.failed !== null
                  ? `sent at ${call.called} ms from the navigation, failed at ${call.failed} ms (${call.error})`
                  : `sent at ${call.called} ms from the navigation, head not arrived`,
            )
            .join(" | ");
    others = fetches.filter((call) => call.path !== "/events").map(fetchLine).join(", ") || "none";
  } else {
    stream = fetches === null ? "(the page has no note of its fetches)" : fetches;
    others = stream;
  }
  let browser;
  if (Array.isArray(requests)) {
    const events = requests.filter((request) => {
      try {
        return new URL(request.url()).pathname === "/events";
      } catch {
        return false;
      }
    });
    browser =
      events.length === 0
        ? `none of its ${requests.length} latest`
        : events
            .map((request) => {
              const response = request.existingResponse();
              if (response) {
                const head = request.timing().responseStart;
                return `answered ${response.status()}` + (head >= 0 ? `, head ${Math.round(head)} ms after it was sent` : "");
              }
              const failure = request.failure();
              return failure ? `failed (${failure.errorText})` : "no answer";
            })
            .join(" | ");
  } else {
    browser = requests;
  }
  return [
    `live marker: ${word}`,
    `class ${kind}`,
    `message: ${message}`,
    `segments drawn: ${drawn}`,
    `/events: ${stream}`,
    `the browser's /events requests: ${browser}`,
    `the page's other fetches: ${others}`,
    `the view's / from the test: ${view}`,
  ].join("; ");
}

// The one wait for the live marker (#live-word) to read `live`, within expect's timeout
// (story:view-live-marker-under-load). When it is not reached, the error says what the page
// shows and did instead (shows()); a passing wait reads nothing more and makes no request.
export async function expectLive(page) {
  try {
    await expect(page.locator("#live-word")).toHaveText("live");
  } catch (error) {
    const shown = await shows(page);
    error.message += `\nThe page shows: ${shown}`;
    if (typeof error.stack === "string") {
      error.stack = error.stack.replace(/\n    at /, `\nThe page shows: ${shown}\n    at `);
    }
    throw error;
  }
}

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

  page: async ({ page }, use) => {
    await page.addInitScript(noteFetches, FETCHES_KEPT);
    await use(page);
  },

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
