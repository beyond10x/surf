// Adversary cases against story:view-detail-panel: each names the statement it holds the page to.

import http from "node:http";
import { test, expect } from "./fixture.mjs";

const segments = (page) => page.locator("#actions > li.segment");
const selection = (page) => page.locator("#selection");
const rows = (page) => page.locator("#requests > li.request");

async function load(page, address) {
  await page.goto(address);
  await expect(page.locator("#live-word")).toHaveText("live");
}

// The JSON of `value` as surfd writes it: `<`, `>` and `&` as \u escapes.
function toJson(value) {
  return JSON.stringify(value).replace(/[<>&]/g, (c) => "\\u00" + c.charCodeAt(0).toString(16));
}

// A page server of the test's own on 127.0.0.1, a port the system picks: `answer(request,
// response)` answers each request. Closed by the returned close().
async function server(answer) {
  const listener = http.createServer(answer);
  await new Promise((resolve) => listener.listen(0, "127.0.0.1", resolve));
  return {
    origin: "http://127.0.0.1:" + listener.address().port,
    close: () =>
      new Promise((resolve) => {
        listener.closeAllConnections();
        listener.close(resolve);
      }),
  };
}

// Story, timeline.js: "A click on a mark selects that event and stops the event's propagation,
// so the segment's handler does not clear it"; the li.mark is the mark (it "keeps its class and
// its title"). The button covers the mark's short name only: a click on the rest of the mark's
// line, under the same title, reaches the segment's handler.
test("adversary-detail: a click on a mark beside its short name selects that event", async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(2);
  const mark = segments(page).nth(1).locator("li.mark").filter({ hasText: /^ResponseReceived$/ });
  await expect(mark).toBeVisible();
  // #live-word reads "live" once the load of the stream's opening is drawn (view.js, open()), but
  // each later draw of the timeline (an event of the stream) replaces its marks: a mark found and
  // then measured in two steps may be the one taken away. Both boxes are read in one step of the
  // page, from a mark still in the document, and read again until they are.
  let line = null;
  await expect
    .poll(async () => {
      line = await mark.evaluate((node) => {
        const box = node.getBoundingClientRect();
        const button = node.querySelector("button").getBoundingClientRect();
        return node.isConnected ? { width: box.width, height: box.height, beyond: box.right - button.right } : null;
      });
      return line === null ? null : line.beyond;
    }, { message: "the mark's line is wider than its button" })
    .toBeGreaterThan(20);
  await mark.click({ position: { x: line.width - 6, y: line.height / 2 } });
  await expect(
    mark.locator("button"),
    "a click on the mark ResponseReceived, right of its name, does not select the event: it reaches the segment's handler (timeline.js mark)",
  ).toHaveAttribute("aria-pressed", "true");
  await expect(selection(page).locator(".name")).toHaveText("surf.net.ResponseReceived");
});

// docs/view-stream.md: every value comes from pages on the web; crates/fetch media_type(): the
// media type is `type/subtype` of the server's Content-Type, any tokens, in lower case. A server
// the agent visits sends a media type of 4,000 characters without a break: the panel's row must
// keep it inside the panel (brief item 9: the panel does not push the page out of reach).
test("adversary-detail: a long media type from a page's server stays inside the panel", async ({ page, surf }) => {
  const type = "text/" + "x".repeat(4000);
  const pages = await server((request, response) => {
    response.writeHead(200, { "Content-Type": type, "Content-Length": 9 });
    response.end("<p>x</p>\n");
  });
  try {
    const id = await surf.open();
    await surf.agent(id, "go", pages.origin + "/long-type");
    const detail = await surf.api("/api/sessions/" + id);
    const response = detail.entries.find((entry) => entry.name === "surf.net.ResponseReceived");
    expect(response.fields.media_type, "the session kept the server's media type").toBe(type);
    for (const width of [1400, 1000]) {
      await page.setViewportSize({ width, height: 900 });
      await load(page, surf.view);
      await expect(rows(page)).toHaveCount(1);
      expect(await rows(page).nth(0).locator(".media-type").textContent()).toBe(type);
      const measured = await page.evaluate(() => {
        const panel = document.querySelector("aside.detail").getBoundingClientRect();
        const value = document.querySelector("#requests .media-type").getBoundingClientRect();
        return {
          panelRight: Math.round(panel.right),
          valueRight: Math.round(value.right),
          pageWidth: document.documentElement.scrollWidth,
          viewWidth: document.documentElement.clientWidth,
        };
      });
      expect(
        measured.valueRight,
        "at " + width + " px the requests row's media type runs out of the detail panel: " + JSON.stringify(measured),
      ).toBeLessThanOrEqual(measured.panelRight);
      expect(measured.pageWidth, "at " + width + " px the page scrolls sideways: " + JSON.stringify(measured)).toBeLessThanOrEqual(
        measured.viewWidth,
      );
    }
  } finally {
    await pages.close();
  }
});

// Brief item 9: the panel does not push the timeline out of reach. At 1,400 px the panel shares
// the grid's second row with the centre, so the timeline (third row) starts below the requests
// list. A session with 300 requests (300 failed go, built with page.route).
test("adversary-detail: a long requests list leaves the timeline on the first screen", async ({ page, surf }) => {
  const id = "0a1b2c3d-1111-4222-8333-444455556677";
  const now = Date.now();
  const summary = { id, state: "open", address: null, title: null, opened_at: now - 5000, last_action_at: now - 1000, ended_at: null, page: { kind: "none" } };
  const entries = [{ number: 1, session: id, index: 1, kept_at: now, kind: "action", sequence: 1, command: "go", argument: "http://127.0.0.1:9/" }];
  for (let at = 0; at < 300; at += 1) {
    const number = entries.length + 1;
    entries.push({ number, session: id, index: number, kept_at: now, kind: "event", name: "surf.net.FetchFailed", unmapped: false, fields: { request_id: "r" + at, url: "http://127.0.0.1:9/" + at, reason: "Unreachable" } });
  }
  entries.push({ number: entries.length + 1, session: id, index: entries.length + 1, kept_at: now, kind: "answer", page: { kind: "none" }, path: { shown: 1, total: 1, actions: [] } });
  const detail = { session: summary, dropped: 0, log_dropped: false, unreadable: 0, entries };
  const json = (route, value) => route.fulfill({ status: 200, headers: { "Content-Type": "application/json" }, body: toJson(value) });
  await page.route(/\/api\/sessions$/, (route) => json(route, { newest: entries.length, sessions: [summary] }));
  await page.route(/\/api\/sessions\/[0-9a-f-]{36}$/, (route) => json(route, detail));
  await page.route(/\/events$/, (route) => route.fulfill({ status: 200, headers: { "Content-Type": "text/event-stream" }, body: ": nothing\n\n" }));
  await page.goto(surf.view);
  await expect(rows(page)).toHaveCount(300);
  const top = await page.evaluate(() => Math.round(document.querySelector("section.timeline").getBoundingClientRect().top));
  expect(top, "the timeline starts " + top + " px down, below the first screen of 900 px").toBeLessThan(900);
});

// The cases of the fixes (wave 8), each holding a decision of the coordinator's brief.

// A session of the test's own, served by page.route: `entries` as its detail, nothing streamed.
async function routed(page, id, entries) {
  const now = Date.now();
  const summary = { id, state: "open", address: null, title: null, opened_at: now - 5000, last_action_at: now - 1000, ended_at: null, page: { kind: "none" } };
  const detail = { session: summary, dropped: 0, log_dropped: false, unreadable: 0, entries };
  const json = (route, value) => route.fulfill({ status: 200, headers: { "Content-Type": "application/json" }, body: toJson(value) });
  await page.route(/\/api\/sessions$/, (route) => json(route, { newest: entries.length, sessions: [summary] }));
  await page.route(/\/api\/sessions\/[0-9a-f-]{36}$/, (route) => json(route, detail));
  await page.route(/\/events$/, (route) => route.fulfill({ status: 200, headers: { "Content-Type": "text/event-stream" }, body: ": nothing\n\n" }));
}

// Two animation frames: whatever the page changed has been laid out and painted.
function frames(page) {
  return page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}

// The index of the segment that carries the playhead, or -1.
async function playheadAt(page) {
  return segments(page).evaluateAll((nodes) => nodes.findIndex((node) => node.getAttribute("aria-current") === "step"));
}

// Decision 1: a click on a segment outside every mark (the segment's own padding, below its
// outcome) moves the playhead to that segment and clears the selected event.
test("fixes: a click on a segment outside every mark clears the selected event", async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(3);
  const mark = segments(page).nth(1).locator("li.mark").filter({ hasText: /^ResponseReceived$/ });
  await mark.locator("button").click();
  await expect(mark.locator("button")).toHaveAttribute("aria-pressed", "true");
  await expect(selection(page).locator(".name")).toHaveText("surf.net.ResponseReceived");
  const third = segments(page).nth(2);
  await third.scrollIntoViewIfNeeded();
  await expect(third).toBeVisible();
  // The segment's box and what is at the position are read in one step of the page, from a
  // segment still in the document (the page may draw the timeline again: see the case above).
  let position = null;
  await expect
    .poll(async () => {
      const spot = await third.evaluate((node) => {
        const frame = node.getBoundingClientRect();
        const at = { x: 4, y: frame.height - 4 };
        const target = document.elementFromPoint(frame.x + at.x, frame.y + at.y);
        return { at, hit: node.isConnected && target === node };
      });
      position = spot.at;
      return spot.hit;
    }, { message: "the position is the segment itself, no mark and no button" })
    .toBe(true);
  await third.click({ position });
  await expect.poll(() => playheadAt(page)).toBe(2);
  await expect(mark.locator("button")).toHaveAttribute("aria-pressed", "false");
  await expect(selection(page).locator(".name")).toHaveCount(0);
  await expect(selection(page).locator(".sequence")).toHaveText("3");
  await expect.poll(() => page.evaluate(() => location.hash)).toBe("#session=" + id + "&at=3");
});

// Decision 1: Enter and Space on a focused mark's button select its event once (one render of
// the timeline, counted by the clears of #actions) and keep the focus on that button.
test("fixes: Enter and Space on a focused mark's button select its event once", async ({ page, surf, record }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  // /events is never answered: no stream opens, so open() does not load and render the timeline
  // again between the two reads of the count. The word stays "connecting" without an open
  // stream, so the first wait is the two segments of the first load.
  await page.route(/\/events$/, () => {});
  await page.goto(surf.view);
  await expect(segments(page)).toHaveCount(2);
  await page.evaluate(() => {
    window.surfTestRenders = 0;
    new MutationObserver((records) => {
      window.surfTestRenders += records.filter((record) => record.removedNodes.length > 0).length;
    }).observe(document.getElementById("actions"), { childList: true });
  });
  const renders = () => page.evaluate(() => window.surfTestRenders);
  for (const [key, short, name] of [
    ["Enter", "ResponseReceived", "surf.net.ResponseReceived"],
    [" ", "RequestStarted", "surf.net.RequestStarted"],
  ]) {
    const button = segments(page).nth(1).locator("li.mark").filter({ hasText: new RegExp("^" + short + "$") }).locator("button");
    const number = await button.getAttribute("data-number");
    await button.focus();
    const before = await renders();
    await page.keyboard.press(key === " " ? "Space" : key);
    await expect(button, key + " on the focused button selects its event").toHaveAttribute("aria-pressed", "true");
    await expect(selection(page).locator(".name")).toHaveText(name);
    await frames(page);
    expect(await renders(), key + " selects once: one render of the timeline").toBe(before + 1);
    const focused = await page.evaluate(() => {
      const node = document.activeElement;
      return node ? node.tagName.toLowerCase() + " " + node.getAttribute("class") + " " + node.getAttribute("data-number") : null;
    });
    expect(focused, key + " keeps the focus on the mark's button").toBe("button mark-select " + number);
  }
  expect(record.requestsOf("/api/sessions"), "the page loaded once: /events is held, so no stream opened").toHaveLength(1);
});

// Decision 2: every value the panel writes wraps inside it. A session whose action has a
// command and an argument, an event a name, a field name and a field value, and a request a
// reason, each of 4,000 characters without a break: no horizontal scroll at 1,400 and 1,000 px,
// with the action and with the event selected. The answer's page line (the centre's) fails with
// a reason of 4,000 characters too: no value of a session scrolls the page sideways.
test("fixes: no horizontal scroll from a long reason, event name, field name and value", async ({ page, surf }) => {
  const id = "0a1b2c3d-1111-4222-8333-444455556688";
  const now = Date.now();
  const long = (c) => c.repeat(4000);
  const base = { session: id, kept_at: now };
  const entries = [
    { ...base, number: 1, index: 1, kind: "action", sequence: 1, command: long("c"), argument: "http://127.0.0.1:9/" + long("a") },
    { ...base, number: 2, index: 2, kind: "event", name: "surf.net." + long("N"), unmapped: false, fields: { [long("f")]: long("v"), request_id: "r0" } },
    { ...base, number: 3, index: 3, kind: "event", name: "surf.net.FetchFailed", unmapped: false, fields: { request_id: "r0", url: "http://127.0.0.1:9/", reason: long("R") } },
    { ...base, number: 4, index: 4, kind: "answer", page: { kind: "failed", reason: long("P"), address: "http://127.0.0.1:9/" }, path: { shown: 1, total: 1, actions: [] } },
  ];
  await routed(page, id, entries);
  for (const width of [1400, 1000]) {
    await page.setViewportSize({ width, height: 900 });
    await page.goto(surf.view);
    await expect(segments(page)).toHaveCount(1);
    await expect(rows(page)).toHaveCount(1);
    for (const selected of ["action", "event"]) {
      if (selected === "event") {
        await page.locator('button.mark-select[data-number="2"]').click();
        await expect(selection(page).locator(".name")).toHaveText("surf.net." + long("N"));
      } else {
        // The address keeps the selection of the width before; the segment selects its action.
        await segments(page).nth(0).locator("button.select").click();
        await expect(selection(page).locator(".command")).toHaveText(long("c"));
      }
      const measured = await page.evaluate(() => {
        const panel = document.querySelector("aside.detail").getBoundingClientRect();
        const values = [...document.querySelectorAll("aside.detail #selection *, aside.detail #requests *")];
        const outside = values
          .filter((node) => Math.round(node.getBoundingClientRect().right) > Math.round(panel.right))
          .map((node) => node.tagName.toLowerCase() + "." + node.getAttribute("class"));
        return {
          outside,
          pageWidth: document.documentElement.scrollWidth,
          viewWidth: document.documentElement.clientWidth,
        };
      });
      expect(measured.outside, "at " + width + " px with the " + selected + " selected, nodes past the panel's right edge").toEqual([]);
      expect(measured.pageWidth, "at " + width + " px with the " + selected + " selected the page scrolls sideways").toBeLessThanOrEqual(
        measured.viewWidth,
      );
    }
  }
});

// Decision 3: the requests list scrolls inside itself. Scrolled to its end, a new row is in
// view; scrolled up, the rows the reader sees do not move when one is added.
test("fixes: the requests list follows a new row at its end and stays where a reader scrolled", async ({ page, surf }) => {
  test.setTimeout(60_000);
  const id = await surf.open();
  for (let at = 0; at < 12; at += 1) {
    await surf.agent(id, "go", surf.pages + (at % 2 === 0 ? "/first/a.html" : "/first/b.html"));
  }
  await load(page, surf.view);
  await expect(rows(page)).toHaveCount(12);
  // Whether the list is a scroller that holds more than it shows, and where its rows are.
  const geometry = () =>
    page.evaluate(() => {
      const list = document.getElementById("requests");
      let box = list;
      while (box && box.scrollHeight <= box.clientHeight + 1) {
        box = box.parentElement;
      }
      const frame = (box || list).getBoundingClientRect();
      const all = [...list.querySelectorAll(":scope > li.request")];
      const last = all[all.length - 1].getBoundingClientRect();
      const firstShown = all.findIndex((row) => row.getBoundingClientRect().bottom > frame.top + 1);
      return {
        scroller: box ? box.tagName.toLowerCase() + "." + box.getAttribute("class") : null,
        lastInView: last.top >= frame.top - 1 && last.bottom <= frame.bottom + 1,
        firstShown,
        firstTop: all[firstShown].getBoundingClientRect().top - frame.top,
      };
    });
  await frames(page);
  const atLoad = await geometry();
  expect(atLoad.scroller, "the list scrolls inside the panel, not the page").not.toBe("html.null");
  expect(atLoad.scroller).not.toBeNull();
  expect(atLoad.lastInView, "at load the list shows its newest row: " + JSON.stringify(atLoad)).toBe(true);
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await expect(rows(page)).toHaveCount(13);
  await expect.poll(async () => (await geometry()).lastInView, "a list at its end follows the new row").toBe(true);
  await page.locator("#requests > li.request").first().hover();
  await page.mouse.wheel(0, -100_000);
  await expect.poll(async () => (await geometry()).firstShown, "the reader scrolled to the oldest row").toBe(0);
  await frames(page);
  const before = await geometry();
  expect(before.lastInView).toBe(false);
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  await expect(rows(page)).toHaveCount(14);
  await frames(page);
  const after = await geometry();
  const moved = "a list scrolled up does not move: " + JSON.stringify({ before, after });
  expect(after.firstShown, moved).toBe(before.firstShown);
  // Within a pixel: the browser's scroll anchoring rounds; a row is about 40 px high.
  expect(Math.abs(after.firstTop - before.firstTop), moved).toBeLessThan(1.5);
  expect(after.lastInView).toBe(false);
});

// Decision 3, the selection part: an event with 200 fields, one of them 100,000 characters,
// selected at 1,400 x 900, leaves the timeline on the first screen.
test("fixes: an event with 200 fields leaves the timeline on the first screen", async ({ page, surf }) => {
  const id = "0a1b2c3d-1111-4222-8333-444455556699";
  const now = Date.now();
  const fields = {};
  for (let at = 0; at < 200; at += 1) {
    fields["field_" + at] = at === 100 ? "x".repeat(100_000) : "value " + at;
  }
  const base = { session: id, kept_at: now };
  await routed(page, id, [
    { ...base, number: 1, index: 1, kind: "action", sequence: 1, command: "go", argument: "http://127.0.0.1:9/" },
    { ...base, number: 2, index: 2, kind: "event", name: "surf.net.Many", unmapped: false, fields },
    { ...base, number: 3, index: 3, kind: "answer", page: { kind: "none" }, path: { shown: 1, total: 1, actions: [] } },
  ]);
  await page.goto(surf.view);
  await expect(segments(page)).toHaveCount(1);
  await page.locator('button.mark-select[data-number="2"]').click();
  await expect(selection(page).locator("dl.fields dt")).toHaveCount(200);
  const top = await page.evaluate(() => Math.round(document.querySelector("section.timeline").getBoundingClientRect().top + window.scrollY));
  expect(top, "the timeline starts " + top + " px down the page, below the first screen of 900 px").toBeLessThan(900);
});
