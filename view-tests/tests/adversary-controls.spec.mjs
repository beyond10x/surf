// Adversary cases against story:view-controls-and-changes, each against the real session
// (surf-view-fixture runs surf-session) on a page server of the test's own on 127.0.0.1. Each
// names the statement it holds the view to.

import http from "node:http";
import { test, expect } from "./fixture.mjs";

const segments = (page) => page.locator("#actions > li.segment");
const changes = (page) => page.locator("#changes li.change");

async function load(page, address) {
  await page.goto(address);
  await expect(page.locator("#live-word")).toHaveText("live");
}

async function playheadTo(page, at) {
  await segments(page).nth(at).locator("button.select").click();
  await expect(segments(page).nth(at)).toHaveAttribute("aria-current", "step");
}

async function changed(page) {
  return changes(page).evaluateAll((nodes) => nodes.map((node) => node.textContent));
}

// [handle, kind, label, value, flags] of every element of the group "fields".
async function fields(page) {
  return page.locator("#now section.now-group").evaluateAll((sections) =>
    sections
      .filter((section) => section.querySelector("h4").textContent === "fields")
      .flatMap((section) =>
        [...section.querySelectorAll("li.now-element")].map((node) => {
          const text = (selector) => {
            const found = node.querySelector(selector);
            return found ? found.textContent : null;
          };
          const flags = [...node.querySelectorAll(".flag")].map((flag) => flag.textContent).join(" ");
          return [text(".handle"), text(".kind"), text(".label"), text(".value"), flags];
        }),
      ),
  );
}

// A page server of the test's own on 127.0.0.1: `pages` maps a path to its HTML; a POST's body
// is kept in `posted`, and its answer is `pages[path]`.
async function server(pages) {
  const posted = [];
  const listener = http.createServer((request, response) => {
    const chunks = [];
    request.on("data", (chunk) => chunks.push(chunk));
    request.on("end", () => {
      const path = new URL(request.url, "http://127.0.0.1").pathname;
      const body = Buffer.concat(chunks).toString("utf8");
      if (request.method === "POST") {
        posted.push(body);
      }
      const html = pages[path];
      if (html === undefined) {
        response.writeHead(404, { "Content-Type": "text/plain" });
        response.end("not found");
        return;
      }
      response.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
      response.end(html);
    });
  });
  await new Promise((resolve) => listener.listen(0, "127.0.0.1", resolve));
  return {
    origin: "http://127.0.0.1:" + listener.address().port,
    posted,
    close: () =>
      new Promise((resolve) => {
        listener.closeAllConnections();
        listener.close(resolve);
      }),
  };
}

// The view's /events read from Node with the view token: every byte it sent so far in `held.text`.
async function stream(surf) {
  const controller = new AbortController();
  const answer = await fetch(surf.origin + "/events", {
    headers: { Authorization: "Bearer " + surf.token },
    signal: controller.signal,
  });
  expect(answer.status).toBe(200);
  const reader = answer.body.getReader();
  const decoder = new TextDecoder();
  const held = { text: "" };
  const reading = (async () => {
    try {
      for (;;) {
        const { value, done } = await reader.read();
        if (done) {
          return;
        }
        held.text += decoder.decode(value, { stream: true });
      }
    } catch {
      // Aborted at the end of the test.
    }
  })();
  return {
    held,
    close: async () => {
      controller.abort();
      await reading;
    },
  };
}

// Nothing of the page holds any of `fragments`: its markup, its text, its address.
async function holdsNowhere(page, fragments) {
  const markup = await page.content();
  const text = await page.evaluate(() => document.documentElement.textContent);
  for (const fragment of fragments) {
    expect(markup).not.toContain(fragment);
    expect(text).not.toContain(fragment);
    expect(page.url()).not.toContain(fragment);
  }
}

// A form posted to /done: a readonly and a disabled text control, a password whose type is
// written in capitals, a checkbox, a select and a submit button.
const FORM = `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Form</title></head><body>
<form action="/done" method="post">
<p><label>Code <input type="text" name="code" value="abc" readonly></label></p>
<p><label>Off <input type="text" name="off" disabled></label></p>
<p><label>Secret <input type="PASSWORD" name="pw"></label></p>
<p><label><input type="checkbox" name="keep"> Keep</label></p>
<p><label>Pick <select name="pick"><option>One</option><option>Two</option></select></label></p>
<p><button type="submit">Send</button></p>
</form></body></html>`;

const DONE = `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Done</title></head><body><p>Thanks</p></body></html>`;

// The element `handle` of the `now` member of the newest answer entry of the detail.
function nowOf(detail, handle) {
  const answers = detail.entries.filter((entry) => entry.kind === "answer" && entry.now);
  return answers[answers.length - 1].now.elements.find((element) => element.handle === handle);
}

function changedOfLast(detail) {
  const answers = detail.entries.filter((entry) => entry.kind === "answer");
  return answers[answers.length - 1].changed;
}

// docs/view-stream.md, the now member: "`disabled`, `checked` (...), `required`, `readonly`: `true`
// when the line writes the flag". Each flag reaches the JSON and the panel "Now" as itself: a
// readonly control is not disabled, a disabled one is not readonly. No other page test has
// either flag, so surfd's control_state() and now.js's FLAGS could swap them unseen.
test("adversary-controls: a readonly and a disabled control each show their own flag in the JSON and the panel", async ({ page, surf }) => {
  const site = await server({ "/form.html": FORM, "/done": DONE });
  try {
    const id = await surf.open();
    expect(await surf.agent(id, "go", site.origin + "/form.html")).toBe(0);
    await load(page, surf.view);
    await expect(segments(page)).toHaveCount(2);
    const detail = await surf.api("/api/sessions/" + id);
    const code = nowOf(detail, "f1").state;
    const off = nowOf(detail, "f2").state;
    expect([code.readonly, code.disabled]).toEqual([true, false]);
    expect([off.disabled, off.readonly]).toEqual([true, false]);
    await expect.poll(() => fields(page)).toEqual([
      ["f1", "text", "Code", '"abc"', "readonly"],
      ["f2", "text", "Off", '""', "disabled"],
      ["f3", "password", "Secret", "(empty)", ""],
      ["f4", "checkbox", "Keep", null, ""],
      ["f5", "select", "Pick", '"One"', ""],
    ]);
  } finally {
    await site.close();
  }
});

// docs/view-stream.md, the changed member: for `checked`, `before` and `after` `true` or `false`;
// for `selected`, the labels before and after. A pick turns the checkbox on and moves the select:
// the JSON and the panel give the side before first. No other test has a checked or a selected
// line from a session, so surfd's changed() and changes.js could swap the sides unseen.
test("adversary-controls: a picked checkbox and select show before then after, in the JSON and the panel", async ({ page, surf }) => {
  const site = await server({ "/form.html": FORM, "/done": DONE });
  try {
    const id = await surf.open();
    expect(await surf.agent(id, "go", site.origin + "/form.html")).toBe(0);
    await load(page, surf.view);
    expect(await surf.agent(id, "pick", "f4", "on")).toBe(0);
    await expect(segments(page)).toHaveCount(3);
    expect(changedOfLast(await surf.api("/api/sessions/" + id))).toEqual({
      kind: "controls",
      lines: [{ handle: "f4", what: "checked", before: false, after: true }],
    });
    await expect(page.locator("#changes-heading")).toHaveText("Changed by pick f4");
    await expect.poll(() => changed(page)).toEqual(["f4 checked false -> true"]);
    expect(await surf.agent(id, "pick", "f5", "Two")).toBe(0);
    await expect(segments(page)).toHaveCount(4);
    expect(changedOfLast(await surf.api("/api/sessions/" + id))).toEqual({
      kind: "controls",
      lines: [
        {
          handle: "f5",
          what: "selected",
          before: { kind: "text", text: "One", cut: false },
          after: { kind: "text", text: "Two", cut: false },
        },
      ],
    });
    await expect.poll(() => changed(page)).toEqual(['f5 selected "One" -> "Two"']);
    await expect.poll(async () => (await fields(page))[3]).toEqual(["f4", "checkbox", "Keep", null, "checked"]);
  } finally {
    await site.close();
  }
});

// docs/view-stream.md: the length shown for a password is "the `value` of the newest
// `FillRequested` entry of that handle since the answer that loaded the document". Three fills of
// one password, 12 bytes, 3 bytes, then empty: each changed line gives the length before and
// after that action, and the playhead on an earlier fill shows that fill's length, not a later
// one. Case 6 has one fill, whose side before is "(empty)" whatever the lengths hold.
test("adversary-controls: refilling a password shows the length before and after, and the playhead keeps each fill's length", async ({ page, surf }) => {
  const site = await server({ "/form.html": FORM, "/done": DONE });
  try {
    const id = await surf.open();
    expect(await surf.agent(id, "go", site.origin + "/form.html")).toBe(0);
    await load(page, surf.view);
    expect(await surf.agentStdin(id, "hunter2-view", "fill", "f3", "-")).toBe(0);
    expect(await surf.agentStdin(id, "abc", "fill", "f3", "-")).toBe(0);
    await expect(segments(page)).toHaveCount(4);
    await expect(page.locator("#changes-heading")).toHaveText("Changed by fill f3");
    await expect.poll(() => changed(page)).toEqual(["f3 value 12 bytes -> 3 bytes"]);
    expect((await fields(page))[2]).toEqual(["f3", "password", "Secret", "3 bytes", ""]);
    expect(await surf.agentStdin(id, "", "fill", "f3", "-")).toBe(0);
    await expect(segments(page)).toHaveCount(5);
    await expect.poll(() => changed(page)).toEqual(["f3 value 3 bytes -> (empty)"]);
    expect((await fields(page))[2]).toEqual(["f3", "password", "Secret", "(empty)", ""]);
    await playheadTo(page, 2);
    await expect(page.locator("#changes-heading")).toHaveText("Changed by fill f3");
    await expect.poll(() => changed(page)).toEqual(["f3 value (empty) -> 12 bytes"]);
    expect((await fields(page))[2]).toEqual(["f3", "password", "Secret", "12 bytes", ""]);
    await playheadTo(page, 1);
    expect((await fields(page))[2]).toEqual(["f3", "password", "Secret", "(empty)", ""]);
  } finally {
    await site.close();
  }
});

// Case 2's claim, against the real session (case 2 runs against surfd's stand-in): after two
// fills of a password, one with a quote and markup, and a press that posts it, no byte of the
// stream since before the session opened, of the detail, of the list or of the page holds
// either password, the request's fields shown in the detail panel included. The server got
// both, so the session did send them. Each password is checked by two pieces of letters and
// digits that JSON and the answer write unchanged, so an escaped copy is found too.
test("adversary-controls: after two fills and a press the real session's stream, detail and page hold no byte of either password", async ({ page, surf }) => {
  const site = await server({ "/form.html": FORM, "/done": DONE });
  const events = await stream(surf);
  try {
    const id = await surf.open();
    expect(await surf.agent(id, "go", site.origin + "/form.html")).toBe(0);
    await load(page, surf.view);
    expect(await surf.agentStdin(id, "hunter2-view", "fill", "f3", "-")).toBe(0);
    expect(await surf.agentStdin(id, 'Qx7"<b>Wv9&amp;', "fill", "f3", "-")).toBe(0);
    expect(await surf.agent(id, "press", "b1")).toBe(0);
    expect(site.posted).toHaveLength(1);
    expect(site.posted[0]).toContain("Qx7");
    await expect(segments(page)).toHaveCount(5);
    await expect.poll(() => events.held.text.includes('"to":"' + site.origin + '/done"')).toBe(true);
    const fragments = ["hunter2", "2-view", "Qx7", "Wv9"];
    const detail = await surf.apiText("/api/sessions/" + id);
    const list = await surf.apiText("/api/sessions");
    for (const fragment of fragments) {
      expect(events.held.text).not.toContain(fragment);
      expect(detail).not.toContain(fragment);
      expect(list).not.toContain(fragment);
    }
    expect(detail).toContain('"value":"12 bytes"');
    await holdsNowhere(page, fragments);
    const press = segments(page).nth(4);
    for (const name of ["FormSubmitted", "RequestStarted"]) {
      await press.locator("li.mark").filter({ hasText: new RegExp("^" + name + "$") }).locator("button").click();
      await expect(page.locator("#selection")).toContainText(name);
      await holdsNowhere(page, fragments);
    }
  } finally {
    await events.close();
    await site.close();
  }
});

// Case 7 for the places this story added beyond a control's label and value and a text line: an
// option's label in a selected line, a button's label in the buttons group, and a checkbox's
// label, each holding a quote and markup, are text: no script or img element, no dialog.
test("adversary-controls: markup in an option, a button and a checkbox label is text in both panels", async ({ page, surf, record }) => {
  const forged = '"&lt;img src=x onerror=alert(1)&gt;&lt;script&gt;alert(2)&lt;/script&gt;';
  const shown = '"<img src=x onerror=alert(1)><script>alert(2)</script>';
  const html = `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Forged</title></head><body>
<form action="/done" method="post">
<p><label>Pick <select name="s"><option>${forged}</option><option>B${forged}</option></select></label></p>
<p><label><input type="checkbox" name="c"> ${forged}</label></p>
<p><button type="submit">${forged}</button></p>
</form></body></html>`;
  const site = await server({ "/forged.html": html, "/done": DONE });
  try {
    const id = await surf.open();
    expect(await surf.agent(id, "go", site.origin + "/forged.html")).toBe(0);
    await load(page, surf.view);
    expect(await surf.agent(id, "pick", "f1", "B" + shown)).toBe(0);
    await expect(segments(page)).toHaveCount(3);
    await expect.poll(() => changed(page)).toEqual(['f1 selected "' + shown + '" -> "B' + shown + '"']);
    await expect(page.locator("#now .now-element .label")).toHaveText(["Pick", shown, shown]);
    expect(await page.locator("img").count()).toBe(0);
    expect(await page.locator("script").count()).toBe(1);
    expect(record.dialogs).toEqual([]);
    expect(record.errors).toEqual([]);
    expect(record.violations).toEqual([]);
  } finally {
    await site.close();
  }
});

// The changed panel with Latest off on a fill, across a reset of the stream (docs/view-stream.md,
// "The stream": forget() drops what is loaded and the detail is asked for again): the panel and
// the password's length come back as they were, and a new action does not move them.
test("adversary-controls: with Latest off the changed panel and the length stay on the fill across a reset", async ({ page, surf }) => {
  const site = await server({ "/form.html": FORM, "/done": DONE });
  try {
    const id = await surf.open();
    expect(await surf.agent(id, "go", site.origin + "/form.html")).toBe(0);
    expect(await surf.agentStdin(id, "hunter2-view", "fill", "f3", "-")).toBe(0);
    expect(await surf.agent(id, "pick", "f4", "on")).toBe(0);
    await load(page, surf.piped);
    await expect(segments(page)).toHaveCount(4);
    await playheadTo(page, 2);
    await expect.poll(() => changed(page)).toEqual(["f3 value (empty) -> 12 bytes"]);
    await surf.cut();
    await expect(page.locator("#live-word")).toHaveText("reconnecting", { timeout: 5000 });
    expect(await surf.agentStdin(id, "abc", "fill", "f3", "-")).toBe(0);
    await surf.mend();
    await expect(page.locator("#live-word")).toHaveText("live");
    await expect(segments(page)).toHaveCount(5);
    await expect(segments(page).nth(2)).toHaveAttribute("aria-current", "step");
    await expect(page.locator("#changes-heading")).toHaveText("Changed by fill f3");
    await expect.poll(() => changed(page)).toEqual(["f3 value (empty) -> 12 bytes"]);
    expect((await fields(page))[2]).toEqual(["f3", "password", "Secret", "12 bytes", ""]);
  } finally {
    await site.close();
  }
});
