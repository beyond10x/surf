// Adversary cases against story:view-first-page, and the cases of the fixes that answer them:
// each names the statement it holds.

import { test, expect } from "./fixture.mjs";

const HOSTILE = '"><img src=x onerror=alert(1)>';
const REFUSED = "The view refused the token. Open the address the program printed, with its #token.";

const items = (page) => page.locator("#session-list > li");
const segments = (page) => page.locator("#actions > li.segment");
const latest = (page) => page.locator("#latest");
const pageAddress = (page) => page.locator("#page-line .part.address dd, #page-line .line.failed .address");

async function load(page, address) {
  await page.goto(address);
  await expect(page.locator("#live-word")).toHaveText("live");
}

// The index of the segment that carries the playhead, or -1.
async function playheadAt(page) {
  return segments(page).evaluateAll((nodes) => nodes.findIndex((node) => node.getAttribute("aria-current") === "step"));
}

async function texts(locator) {
  return locator.evaluateAll((nodes) => nodes.map((node) => node.textContent));
}

// The JSON of `value` as surf_view::to_json writes it: `<`, `>` and `&` as \u escapes.
function toJson(value) {
  return JSON.stringify(value).replace(/[<>&]/g, (c) => "\\u00" + c.charCodeAt(0).toString(16));
}

// Holds back every answer of /api/sessions/<id> (asked of surfd at once) until release().
async function holdDetail(page, id) {
  const held = { asked: 0, release: null };
  const released = new Promise((resolve) => {
    held.release = resolve;
  });
  await page.route("**/api/sessions/" + id, async (route) => {
    const response = await route.fetch();
    held.asked += 1;
    await released;
    await route.fulfill({ response });
  });
  return held;
}

// state.js setDetail: "Entries already held above the detail's newest are kept, so that a stream
// entry read while the detail loaded is not lost"; story case 7: with Latest on, a go adds a
// segment and shows its page line. A session selected while its agent acts: the entries the
// stream delivers before the detail's answer arrives are held and merged by setDetail.
test("adversary: an action streamed while the selected session's detail loads is not lost", async ({ page, surf }) => {
  const first = await surf.open();
  const second = await surf.open();
  await load(page, surf.view);
  await expect(items(page)).toHaveCount(2);
  const held = await holdDetail(page, second);
  await items(page).nth(1).locator("button").click();
  await expect.poll(() => held.asked).toBe(1);
  await surf.agent(second, "go", surf.pages + "/first/a.html");
  // The session event after the go's answer entry has reached the page: the list shows the address.
  await expect(items(page).nth(1).locator(".address")).toHaveText(surf.pages + "/first/a.html");
  held.release();
  await expect(
    segments(page),
    "the go of the selected session, read from the stream while its detail loaded, is lost (state.js addEntry/setDetail)",
  ).toHaveCount(2, { timeout: 5000 });
  await expect(pageAddress(page)).toHaveText(surf.pages + "/first/a.html");
  expect(first).not.toBe(second);
});

// The same after a reset (docs/view-stream.md, "The stream"): forget() drops what is loaded and
// the selected session's detail is asked for again; a go streamed while it loads is not lost.
test("adversary: an action streamed while the detail loads after a reset is not lost", async ({ page, surf, record }) => {
  test.setTimeout(120_000);
  const id = await surf.open();
  await load(page, surf.piped);
  await expect(segments(page)).toHaveCount(1);
  await surf.cut();
  await expect(page.locator("#live-word")).toHaveText("reconnecting", { timeout: 5000 });
  for (let at = 0; at < 25; at += 1) {
    await surf.agent(id, "go", surf.pages + (at % 2 === 0 ? "/first/a.html" : "/first/b.html"));
  }
  const held = await holdDetail(page, id);
  await surf.mend();
  // One detail for the new stream, one for its reset: forget() has run.
  await expect.poll(() => held.asked, { timeout: 30_000 }).toBe(2);
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  // The session event after the go's answer entry has reached the page: the list shows the address.
  await expect(items(page).nth(0).locator(".address")).toHaveText(surf.pages + "/first/b.html");
  // The two held answers: the load of the opening and the load of the reset.
  const answers = record.requestsOf("/api/sessions/" + id).slice(-2);
  held.release();
  // The state is read once both loads are in: live (the load of the opening is drawn), the end
  // of both answers, and two frames; not a deadline from the release (story D287).
  await expect(page.locator("#live-word")).toHaveText("live");
  await Promise.all(answers.map((answer) => answer.body));
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  expect(await segments(page).count(), "the go streamed while the detail loaded after the reset is lost").toBe(27);
  expect(await texts(segments(page).locator(".sequence"))).toEqual(Array.from({ length: 27 }, (_, at) => String(at + 1)));
  await expect(pageAddress(page)).toHaveText(surf.pages + "/first/b.html");
});

// Story, timeline.js: "with Latest off the playhead stays and a marker reads `<n> newer`"; the
// coordinator's statement for an action the bound dropped: the playhead moves once to the oldest
// action held, Latest stays off, the marker counts the actions newer than it, and the fragment's
// `at` is written once with its sequence.
test("adversary: with Latest off the playhead stays after its action is dropped by the bound", async ({ page, startFixture, record }) => {
  test.setTimeout(120_000);
  const surf = await startFixture(["--entries-per-session", "30"]);
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await load(page, surf.piped);
  await expect(segments(page)).toHaveCount(2);
  await segments(page).nth(0).locator("button.select").click();
  await expect(latest(page)).toHaveAttribute("aria-pressed", "false");
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  // open and three go held, the playhead on the open.
  await expect(segments(page)).toHaveCount(4);
  await expect(page.locator("#newer")).toHaveText("3 newer");
  const detail = await surf.api("/api/sessions/" + id);
  expect(detail.dropped, "the open's entries are dropped by the bound").toBeGreaterThan(6);
  const kept = detail.entries.filter((entry) => entry.kind === "action");
  expect(kept[0].sequence, "the action at the playhead is no longer held").toBeGreaterThan(1);
  const oldest = "#session=" + id + "&at=" + kept[0].sequence;
  const cutAt = record.addresses.length;
  await surf.cut();
  await expect(page.locator("#live-word")).toHaveText("reconnecting", { timeout: 5000 });
  await surf.mend();
  await expect(page.locator("#live-word")).toHaveText("live");
  await expect(page.locator("#actions > li.gap")).toHaveText(/ dropped$/);
  await expect(segments(page)).toHaveCount(kept.length);
  await expect.poll(() => playheadAt(page), "the playhead is on the oldest action held").toBe(0);
  await expect(latest(page)).toHaveAttribute("aria-pressed", "false");
  await expect(page.locator("#newer")).toHaveText(kept.length - 1 + " newer");
  await expect.poll(() => page.evaluate(() => location.hash)).toBe(oldest);
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  await expect(segments(page)).toHaveCount(kept.length + 1);
  await expect(page.locator("#newer")).toHaveText(kept.length + " newer");
  expect(await playheadAt(page), "a further go leaves the playhead on the oldest action held").toBe(0);
  await expect(latest(page)).toHaveAttribute("aria-pressed", "false");
  expect(await page.evaluate(() => location.hash)).toBe(oldest);
  const written = record.addresses.slice(cutAt).filter((address) => address.includes("&at="));
  expect(written.length, "the fragment's at is written once: " + JSON.stringify(written)).toBe(1);
  expect(written[0].endsWith(oldest)).toBe(true);
});

// docs/design/live-view.md item 6: the token is valid until surfd exits. A stream refused with
// 401 after the page loaded is not asked again (another program may listen on the old port): the
// page reads offline, says the token was refused, and makes no further request.
test("adversary: a refused token on the stream stops the page", async ({ page, surf, record }) => {
  test.setTimeout(60_000);
  await surf.open();
  await load(page, surf.piped);
  await expect(segments(page)).toHaveCount(1);
  await page.route(/\/events$/, (route) =>
    route.fulfill({
      status: 401,
      headers: { "Content-Type": "application/json", "WWW-Authenticate": "Bearer" },
      body: '{"error":"unauthorized"}',
    }),
  );
  await surf.cut();
  await expect(page.locator("#live-word")).toHaveText("offline", { timeout: 5000 });
  await expect(page.locator("#message")).toBeVisible();
  await expect(page.locator("#message")).toHaveText(REFUSED);
  await surf.mend();
  const refusedAt = record.requests.length;
  await items(page).nth(0).locator("button").click();
  await page.waitForTimeout(12_000);
  expect(record.requests.slice(refusedAt).map((request) => request.path)).toEqual([]);
  await expect(page.locator("#live-word")).toHaveText("offline");
  await expect(page.locator("#message")).toHaveText(REFUSED);
});

// Story, "every value through dom.js and textContent": an event's name and field values go into
// a mark's title attribute as text, never as markup.
test("adversary: hostile event name and field value are a mark's title as text", async ({ page, surf, record }) => {
  const id = "0a1b2c3d-1111-4222-8333-444455556677";
  const now = Date.now();
  const summary = {
    id,
    state: "open",
    address: null,
    title: null,
    opened_at: now - 5000,
    last_action_at: now - 1000,
    ended_at: null,
    page: { kind: "none" },
  };
  const name = "surf.net.Hostile" + HOSTILE;
  const entry = (number, kind, rest) => ({ number, session: id, index: number, kept_at: now - 5000 + number, kind, ...rest });
  const detail = {
    session: summary,
    dropped: 0,
    log_dropped: false,
    unreadable: 0,
    entries: [
      entry(1, "action", { sequence: 1, command: "open", argument: null }),
      entry(2, "event", { name, unmapped: false, fields: { url: HOSTILE } }),
      entry(3, "answer", { page: { kind: "none" }, path: { shown: 1, total: 1, actions: [{ sequence: 1, command: "open", argument: null }] } }),
    ],
  };
  const json = (route, value) =>
    route.fulfill({ status: 200, headers: { "Content-Type": "application/json" }, body: toJson(value) });
  await page.route(/\/api\/sessions$/, (route) => json(route, { newest: 3, sessions: [summary] }));
  await page.route(/\/api\/sessions\/[0-9a-f-]{36}$/, (route) => json(route, detail));
  await page.route(/\/events$/, (route) =>
    route.fulfill({ status: 200, headers: { "Content-Type": "text/event-stream" }, body: ": nothing\n\n" }),
  );
  await page.goto(surf.view);
  await expect(segments(page)).toHaveCount(1);
  const mark = segments(page).nth(0).locator(".mark");
  await expect(mark).toHaveCount(1);
  expect(await mark.getAttribute("title")).toBe(name + "\nurl " + HOSTILE);
  expect(await mark.textContent()).toBe("Hostile" + HOSTILE);
  expect(
    await page.evaluate(() => ({
      script: document.querySelectorAll("script").length,
      img: document.querySelectorAll("img").length,
    })),
  ).toEqual({ script: 1, img: 0 });
  expect(record.errors).toEqual([]);
  expect(record.dialogs).toEqual([]);
});

// dom.js: the only file that sets an attribute or a property of an element; it sets the
// attributes of its list and refuses any other name with an error.
test("adversary: dom.js sets only the attributes of its list, and no other page file sets one", async ({ page, surf }) => {
  await surf.open();
  await load(page, surf.view);
  const result = await page.evaluate(async () => {
    const dom = await import("/dom.js");
    const node = document.createElement("span");
    const refused = [];
    for (const name of ["href", "src", "style", "srcdoc", "onclick", "onerror", "ONLOAD", "formaction", "action", "id", "xlink:href"]) {
      try {
        dom.attribute(node, name, "x");
      } catch {
        refused.push(name);
      }
    }
    const accepted = [];
    for (const name of ["class", "title", "type", "hidden", "aria-current", "aria-pressed", "data-at", "data-domain", "data-number"]) {
      dom.attribute(node, name, "x");
      accepted.push(name);
    }
    let built = true;
    try {
      dom.element("a", { attributes: { href: "x" } });
    } catch {
      built = false;
    }
    return { refused, accepted, set: node.getAttributeNames().sort(), built };
  });
  expect(result.refused).toEqual(["href", "src", "style", "srcdoc", "onclick", "onerror", "ONLOAD", "formaction", "action", "id", "xlink:href"]);
  expect(result.set).toEqual([...result.accepted].sort());
  expect(result.built, "element() with an href").toBe(false);
  const sets = [
    /\.(setAttribute|setAttributeNS|removeAttribute|toggleAttribute|innerHTML|outerHTML|insertAdjacentHTML|textContent|innerText|outerText|className|classList|dataset|style|srcdoc)\b/,
    /\.(src|href|hidden|title|scrollLeft|scrollTop|on[a-z]+)\s*=(?!=)/,
  ];
  for (const name of ["view.js", "state.js", "stream.js", "sessions.js", "page.js", "timeline.js", "detail.js"]) {
    const answer = await fetch(surf.origin + "/" + name);
    expect(answer.status, name).toBe(200);
    const source = await answer.text();
    for (const pattern of sets) {
      expect(source.match(pattern), name + " sets an attribute or a property of an element").toBeNull();
    }
  }
  const answer = await fetch(surf.origin + "/dom.js");
  expect(await answer.text()).toContain("setAttribute");
});
