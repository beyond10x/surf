// The 18 cases of story:view-first-page and the 5 of story:view-live-means-stream-open, each a
// test named after it, each with its own surf-view-fixture (fixture.mjs). Every wait is on the
// thing itself; the deadline is the story's where it names one (2 s, 5 s), otherwise the
// config's 10 s.

import { test, expect, expectLive } from "./fixture.mjs";

const DOMAINS = ["surf.shell", "surf.agent", "surf.navigation", "surf.net", "surf.document", "surf.render"];
const HOSTILE_TITLE = "<img src=x onerror=alert(1)><script>alert(1)</script>";

function domainOf(name) {
  if (typeof name === "string") {
    for (const domain of DOMAINS) {
      if (name.startsWith(domain + ".")) {
        return domain;
      }
    }
  }
  return "other";
}

// The JSON of `value` as surf_view::to_json writes it: `<`, `>` and `&` as \u escapes.
function toJson(value) {
  return JSON.stringify(value).replace(/[<>&]/g, (c) => "\\u00" + c.charCodeAt(0).toString(16));
}

// The detail's entries grouped as actions: each action entry with its events and its answer.
function actionsOf(detail) {
  const result = [];
  let current = null;
  for (const entry of detail.entries) {
    if (entry.kind === "action") {
      current = { action: entry, events: [], answer: null };
      result.push(current);
    } else if (current && entry.kind === "event") {
      current.events.push(entry);
    } else if (current && entry.kind === "answer") {
      current.answer = entry;
      current = null;
    }
  }
  return result;
}

const items = (page) => page.locator("#session-list > li");
const segments = (page) => page.locator("#actions > li.segment");
const playhead = (page) => page.locator('#actions > li.segment[aria-current="step"]');
const latest = (page) => page.locator("#latest");
const pageAddress = (page) => page.locator("#page-line .part.address dd, #page-line .line.failed .address");

async function load(page, address) {
  await page.goto(address);
  await expectLive(page);
}

// The index of the segment that carries the playhead, or -1.
async function playheadAt(page) {
  return page.locator("#actions > li.segment").evaluateAll((nodes) =>
    nodes.findIndex((node) => node.getAttribute("aria-current") === "step"),
  );
}

async function texts(locator) {
  return locator.evaluateAll((nodes) => nodes.map((node) => node.textContent));
}

// Two animation frames: whatever the page changed has been laid out and painted.
function frames(page) {
  return page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}

// Holds the `nth` request (from 1) whose address matches `pattern` until release(); every other
// one goes on. `asked` settles when the held request has arrived.
async function hold(page, pattern, nth) {
  const held = { asked: null, release: null };
  let arrived = null;
  held.asked = new Promise((resolve) => {
    arrived = resolve;
  });
  const released = new Promise((resolve) => {
    held.release = resolve;
  });
  let seen = 0;
  await page.route(pattern, async (route) => {
    seen += 1;
    if (seen === nth) {
      arrived();
      await released;
    }
    await route.continue();
  });
  return held;
}

test("loads without an error", async ({ page, surf, record }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await load(page, surf.view);
  await expect(items(page)).toHaveCount(1);
  await expect(segments(page)).toHaveCount(2);
  await expect(pageAddress(page)).toHaveText(surf.pages + "/first/a.html");
  expect(record.console.filter((message) => message.type === "error")).toEqual([]);
  expect(record.errors).toEqual([]);
  expect(record.violations).toEqual([]);
});

test("lists two sessions", async ({ page, surf }) => {
  const first = await surf.open(surf.pages + "/first/a.html");
  const second = await surf.open(surf.pages + "/first/b.html");
  const list = await surf.api("/api/sessions");
  const summaries = [first, second].map((id) => list.sessions.find((summary) => summary.id === id));
  await load(page, surf.view);
  await expect(items(page)).toHaveCount(2);
  for (const [at, summary] of summaries.entries()) {
    expect(summary.address).not.toBeNull();
    const item = items(page).nth(at);
    await expect(item.locator(".state")).toHaveText("open");
    await expect(item.locator(".address")).toHaveText(summary.address);
    await expect(item.locator(".since")).toHaveText(/^\d+ (s|min) ago$/);
  }
});

test("a new session appears without a reload", async ({ page, surf, record }) => {
  await surf.open(surf.pages + "/first/a.html");
  await load(page, surf.view);
  await expect(items(page)).toHaveCount(1);
  const roots = record.requestsOf("/").length;
  const id = await surf.open(surf.pages + "/first/b.html");
  const summary = (await surf.api("/api/sessions")).sessions.find((s) => s.id === id);
  // Each bound of 2 s below is the sentence's "listed within 2 s", named beside it.
  await expect(items(page)).toHaveCount(2, { timeout: 2000 }); // .engineering/planning/story/view-first-page.md:213-214
  await expect(items(page).last().locator(".address")).toHaveText(summary.address, { timeout: 2000 }); // .engineering/planning/story/view-first-page.md:213-214
  expect(record.requestsOf("/").length).toBe(roots);
});

test("ended sessions stay listed", async ({ page, surf }) => {
  const closing = await surf.open(surf.pages + "/first/a.html");
  const dying = await surf.open(surf.pages + "/first/b.html");
  await load(page, surf.view);
  await expect(items(page)).toHaveCount(2);
  await surf.agent(closing, "close");
  await expect(items(page).nth(0).locator(".state")).toHaveText("closed");
  await expect(items(page).nth(0)).toHaveClass(/(^|\s)ended(\s|$)/);
  await surf.kill(dying);
  await expect(items(page).nth(1).locator(".state")).toHaveText("died");
  await expect(items(page).nth(1)).toHaveClass(/(^|\s)ended(\s|$)/);
  await expect(items(page)).toHaveCount(2);
});

test("page line", async ({ page, surf }) => {
  const loaded = await surf.open();
  await surf.agent(loaded, "go", surf.pages + "/first/a.html");
  const failed = await surf.open();
  await surf.agent(failed, "go", surf.unreachable);
  await load(page, surf.view);
  await expect(items(page)).toHaveCount(2);
  await items(page).nth(0).locator("button").click();
  const line = page.locator("#page-line");
  await expect(line.locator(".line.loaded .part.status dd")).toHaveText("200");
  await expect(line.locator(".line.loaded .part").nth(1).locator("dd")).toHaveText("text/html");
  await expect(line.locator(".line.loaded .part.address dd")).toHaveText(surf.pages + "/first/a.html");
  await items(page).nth(1).locator("button").click();
  await expect(line.locator(".line.failed .word")).toHaveText("failed");
  await expect(line.locator(".line.failed .reason")).toHaveText("Unreachable");
  await expect(line.locator(".line.failed .address")).toHaveText(surf.unreachable);
});

test("timeline of three actions", async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  const detail = await surf.api("/api/sessions/" + id);
  const actions = actionsOf(detail);
  expect(actions.map((action) => action.action.command)).toEqual(["open", "go", "go"]);
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(3);
  expect(await texts(segments(page).locator(".command"))).toEqual(["open", "go", "go"]);
  expect(await texts(segments(page).locator(".sequence"))).toEqual(["1", "2", "3"]);
  for (const [at, action] of actions.entries()) {
    const segment = segments(page).nth(at);
    expect(action.events.length).toBeGreaterThan(0);
    await expect(segment.locator(".mark")).toHaveCount(action.events.length);
    const counts = new Map();
    for (const event of action.events) {
      const domain = domainOf(event.name);
      counts.set(domain, (counts.get(domain) || 0) + 1);
    }
    const order = [...DOMAINS, "other"].filter((domain) => counts.has(domain));
    await expect(segment.locator(".group > .group-label")).toHaveText(order);
    for (const [index, domain] of order.entries()) {
      await expect(segment.locator(".group").nth(index).locator(".mark")).toHaveCount(counts.get(domain));
    }
  }
});

test("latest follows", async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(2);
  await expect(latest(page)).toHaveAttribute("aria-pressed", "true");
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  // Each bound of 2 s below is the sentence's "within 2 s", named beside it.
  await expect(segments(page)).toHaveCount(3, { timeout: 2000 }); // .engineering/planning/story/view-first-page.md:225-226
  await expect(playhead(page)).toHaveCount(1, { timeout: 2000 }); // .engineering/planning/story/view-first-page.md:225-226
  await expect.poll(() => playheadAt(page), { timeout: 2000 }).toBe(2); // .engineering/planning/story/view-first-page.md:225-226
  await expect(pageAddress(page)).toHaveText(surf.pages + "/first/b.html", { timeout: 2000 }); // .engineering/planning/story/view-first-page.md:225-226
});

test("playhead stays", async ({ page, surf }) => {
  const id = await surf.open();
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(1);
  await segments(page).nth(0).locator("button.select").click();
  await expect.poll(() => playheadAt(page)).toBe(0);
  await expect(latest(page)).toHaveAttribute("aria-pressed", "false");
  await expect(page.locator("#page-line .line.none")).toHaveText("none");
  await expect(page.locator("#newer")).toBeHidden();
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  await expect(segments(page)).toHaveCount(2);
  await expect(page.locator("#newer")).toHaveText("1 newer");
  expect(await playheadAt(page)).toBe(0);
  await expect(page.locator("#page-line .line.none")).toHaveText("none");
  await latest(page).click();
  await expect.poll(() => playheadAt(page)).toBe(1);
  await expect(latest(page)).toHaveAttribute("aria-pressed", "true");
  await expect(page.locator("#newer")).toBeHidden();
  await expect(pageAddress(page)).toHaveText(surf.pages + "/first/b.html");
});

test("arrow keys move the playhead", async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(3);
  await expect.poll(() => playheadAt(page)).toBe(2);
  for (const expected of [1, 0, 0]) {
    await page.keyboard.press("ArrowLeft");
    await expect.poll(() => playheadAt(page)).toBe(expected);
    await expect(playhead(page)).toHaveCount(1);
  }
  await expect(latest(page)).toHaveAttribute("aria-pressed", "false");
  for (const expected of [1, 2, 2]) {
    await page.keyboard.press("ArrowRight");
    await expect.poll(() => playheadAt(page)).toBe(expected);
    await expect(playhead(page)).toHaveCount(1);
  }
  await expect(latest(page)).toHaveAttribute("aria-pressed", "false");
});

test("selection in the address", async ({ page, surf, record }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  // The address as the page has it when it asks for the session list, the first thing it does
  // after taking the token, and on every reload and reconnect.
  const hrefs = [];
  await page.route(/\/api\/sessions$/, async (route) => {
    hrefs.push(await page.evaluate(() => location.href));
    await route.continue();
  });
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(3);
  expect(page.url()).not.toContain(surf.token);
  expect(hrefs.length).toBeGreaterThan(0);
  await segments(page).nth(1).locator("button.select").click();
  await expect.poll(() => page.evaluate(() => location.hash)).toBe("#session=" + id + "&at=2");
  expect(await page.evaluate(() => location.href)).not.toContain(surf.token);
  await page.reload();
  await expectLive(page);
  await expect(segments(page)).toHaveCount(3);
  await expect.poll(() => playheadAt(page)).toBe(1);
  await expect(latest(page)).toHaveAttribute("aria-pressed", "false");
  expect(await page.evaluate(() => location.hash)).toBe("#session=" + id + "&at=2");
  expect(await page.evaluate(() => location.href)).not.toContain(surf.token);
  expect(record.addresses.length).toBeGreaterThan(1);
  expect(record.addresses[0]).toContain(surf.token);
  for (const address of record.addresses.slice(1)) {
    expect(address).not.toContain(surf.token);
  }
  for (const href of hrefs) {
    expect(href).not.toContain(surf.token);
  }
});

test("live marker", async ({ page, surf }) => {
  await surf.open();
  await load(page, surf.view);
  await expect(page.locator("#live-time")).toHaveText(/^\d{1,2}:\d{2}:\d{2}$/);
  await surf.stopSurfd();
  await expect(page.locator("#live-word")).toHaveText("reconnecting", { timeout: 5000 });
});

test("hostile address is text", async ({ page, surf, record }) => {
  const id = await surf.open();
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(1);
  const count = () =>
    page.evaluate(() => ({
      script: document.querySelectorAll("script").length,
      img: document.querySelectorAll("img").length,
    }));
  const before = await count();
  const hostile = [
    surf.pages + '/first/a.html?"<script>alert(1)</script>',
    surf.pages + "/first/<img src=x onerror=alert(1)>",
  ];
  for (const [at, address] of hostile.entries()) {
    await surf.agent(id, "go", address);
    await expect(segments(page)).toHaveCount(2 + at);
    await expect(items(page).nth(0).locator(".address")).toHaveText(address, { useInnerText: false });
    expect(await items(page).nth(0).locator(".address").textContent()).toBe(address);
    await expect(pageAddress(page)).toHaveCount(1);
    expect(await pageAddress(page).textContent()).toBe(address);
    expect(await segments(page).last().locator(".argument").textContent()).toBe(address);
  }
  expect(await count()).toEqual(before);
  expect(record.requests.filter((request) => request.path === "/x")).toEqual([]);
  expect(record.dialogs).toEqual([]);
  expect(record.violations).toEqual([]);
});

test("hostile title and unreadable entries are text", async ({ page, surf, record }) => {
  const id = "0a1b2c3d-1111-4222-8333-444455556666";
  const now = Date.now();
  const loaded = { kind: "loaded", status: 200, media_type: "text/html", address: "http://127.0.0.1:9/t.html", title: HOSTILE_TITLE };
  const summary = {
    id,
    state: "open",
    address: "http://127.0.0.1:9/t.html",
    title: HOSTILE_TITLE,
    opened_at: now - 5000,
    last_action_at: now - 1000,
    ended_at: null,
    page: loaded,
  };
  const entry = (number, kind, rest) => ({ number, session: id, index: number, kept_at: now - 5000 + number, kind, ...rest });
  const detail = {
    session: summary,
    dropped: 0,
    log_dropped: false,
    unreadable: 3,
    entries: [
      entry(1, "action", { sequence: 1, command: "open", argument: null }),
      entry(2, "event", { name: "surf.shell.WindowOpened", unmapped: false, fields: { window_id: "2c4e6a80-1b3d-4f57-9a2c-4e6a8b0d2f41" } }),
      entry(3, "answer", { page: { kind: "none" }, path: { shown: 1, total: 1, actions: [{ sequence: 1, command: "open", argument: null }] } }),
      entry(4, "action", { sequence: null, command: null, argument: null }),
      entry(5, "event", { name: null, unmapped: false, fields: {} }),
      entry(6, "answer", { page: loaded, path: { shown: 1, total: 2, actions: [{ sequence: null, command: null, argument: null }] } }),
    ],
  };
  const json = (route, value) =>
    route.fulfill({ status: 200, headers: { "Content-Type": "application/json" }, body: toJson(value) });
  await page.route(/\/api\/sessions$/, (route) => json(route, { newest: 6, sessions: [summary] }));
  await page.route(/\/api\/sessions\/[0-9a-f-]{36}$/, (route) => json(route, detail));
  await page.route(/\/events$/, (route) =>
    route.fulfill({ status: 200, headers: { "Content-Type": "text/event-stream" }, body: ": nothing\n\n" }),
  );
  await page.goto(surf.view);
  await expect(segments(page)).toHaveCount(2);
  expect(await items(page).nth(0).locator(".title").textContent()).toBe(HOSTILE_TITLE);
  expect(await page.locator("#session-heading").textContent()).toBe(HOSTILE_TITLE);
  const title = page.locator("#page-line .line.loaded > .part").nth(3).locator("dd");
  await expect(title).toHaveCount(1);
  expect(await title.textContent()).toBe(HOSTILE_TITLE);
  expect(
    await page.evaluate(() => ({
      script: document.querySelectorAll("script").length,
      img: document.querySelectorAll("img").length,
    })),
  ).toEqual({ script: 1, img: 0 });
  const unreadable = segments(page).nth(1);
  await expect(unreadable.locator(".sequence")).toHaveText("?");
  await expect(unreadable.locator(".command")).toHaveText("?");
  await expect(unreadable.locator(".group > .group-label")).toHaveText(["other"]);
  await expect(unreadable.locator(".group").nth(0).locator(".mark")).toHaveCount(1);
  expect(record.errors).toEqual([]);
  expect(record.dialogs).toEqual([]);
});

test("first stream from newest", async ({ page, surf, record }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await load(page, surf.view);
  await expect.poll(() => record.requestsOf("/events").length).toBeGreaterThan(0);
  const stream = record.requestsOf("/events")[0];
  const lists = record.requestsOf("/api/sessions").filter((request) => request.order < stream.order);
  expect(lists.length).toBeGreaterThan(0);
  const list = JSON.parse(await lists[lists.length - 1].body);
  expect(list.newest).toBeGreaterThan(0);
  expect((await stream.headers)["last-event-id"]).toBe(String(list.newest));
});

test("reconnect after a lost connection", async ({ page, surf, record }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await load(page, surf.piped);
  await expect(segments(page)).toHaveCount(2);
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  await expect(segments(page)).toHaveCount(3);
  await expect(pageAddress(page)).toHaveText(surf.pages + "/first/b.html");
  // Every entry and the session event of that go carry numbers up to newest; the page shows
  // its answer, so the last id it read is newest.
  const lastRead = (await surf.api("/api/sessions")).newest;
  const roots = record.requestsOf("/").length;
  const cutAt = record.requests.length;
  await surf.cut();
  await expect(page.locator("#live-word")).toHaveText("reconnecting", { timeout: 5000 });
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await surf.mend();
  await expect(segments(page)).toHaveCount(4);
  await expect(pageAddress(page)).toHaveText(surf.pages + "/first/a.html");
  await expectLive(page);
  const again = record.requestsOf("/events").filter((request) => request.order >= cutAt);
  expect(again.length).toBeGreaterThan(0);
  for (const request of again) {
    expect((await request.headers)["last-event-id"]).toBe(String(lastRead));
  }
  let answered = null;
  for (const request of again) {
    if ((await Promise.race([request.status, Promise.resolve("pending")])) === 200) {
      answered = request;
      break;
    }
  }
  expect(answered).not.toBeNull();
  await expect
    .poll(() => record.requestsOf("/api/sessions").filter((request) => request.order > answered.order).length)
    .toBeGreaterThan(0);
  expect(await texts(segments(page).locator(".sequence"))).toEqual(["1", "2", "3", "4"]);
  expect(record.requestsOf("/").length).toBe(roots);
});

test("reset after a long absence", async ({ page, surf, record }) => {
  test.setTimeout(60_000);
  const id = await surf.open();
  await load(page, surf.piped);
  await expect(segments(page)).toHaveCount(1);
  await surf.cut();
  await expect(page.locator("#live-word")).toHaveText("reconnecting", { timeout: 5000 });
  for (let at = 0; at < 25; at += 1) {
    await surf.agent(id, "go", surf.pages + (at % 2 === 0 ? "/first/a.html" : "/first/b.html"));
  }
  const mendAt = record.requests.length;
  await surf.mend();
  await expect(segments(page)).toHaveCount(26, { timeout: 30_000 });
  await expectLive(page);
  // Loaded again twice: once as every new stream asks, once more for its `reset`
  // (docs/view-stream.md, "The stream").
  await expect
    .poll(() => record.requestsOf("/api/sessions").filter((request) => request.order >= mendAt).length)
    .toBeGreaterThanOrEqual(2);
  await expect
    .poll(() => record.requestsOf("/api/sessions/" + id).filter((request) => request.order >= mendAt).length)
    .toBeGreaterThanOrEqual(2);
  const expected = Array.from({ length: 26 }, (_, at) => String(at + 1));
  expect(await texts(segments(page).locator(".sequence"))).toEqual(expected);
});

test("dropped entries are a gap", async ({ page, startFixture }) => {
  test.setTimeout(60_000);
  const bound = 30;
  const bounded = await startFixture(["--entries-per-session", String(bound)]);
  const id = await bounded.open();
  for (let at = 0; at < 4; at += 1) {
    await bounded.agent(id, "go", bounded.pages + "/first/a.html");
  }
  const detail = await bounded.api("/api/sessions/" + id);
  // The session numbers its entries from 1, so the newest one's `index` is how many it recorded,
  // whatever a `go` records (`GO_ENTRIES` in crates/session/tests/events.rs). The bound keeps
  // the newest `bound` of them, numbered without a hole, and counts every older one as dropped.
  const indices = detail.entries.map((entry) => entry.index);
  const recorded = indices[indices.length - 1];
  expect(recorded, "open and four go record more entries than the bound").toBeGreaterThan(bound);
  expect(indices).toEqual(Array.from({ length: bound }, (_, at) => recorded - bound + 1 + at));
  expect(detail.dropped).toBe(recorded - bound);
  const kept = detail.entries.filter((entry) => entry.kind === "action").length;
  await load(page, bounded.view);
  await expect(segments(page)).toHaveCount(kept);
  await expect(page.locator("#actions > li").first()).toHaveClass(/(^|\s)gap(\s|$)/);
  await expect(page.locator("#actions > li.gap")).toHaveText(detail.dropped + " dropped");

  const small = await startFixture(["--bytes-total", "4096"]);
  const closed = await small.open();
  await small.agent(closed, "close");
  await expect
    .poll(async () => (await small.api("/api/sessions")).sessions.find((s) => s.id === closed).state)
    .toBe("closed");
  const going = await small.open();
  let dropped = false;
  for (let at = 0; at < 30 && !dropped; at += 1) {
    await small.agent(going, "go", small.pages + "/first/b.html");
    dropped = (await small.api("/api/sessions/" + closed)).log_dropped;
  }
  expect(dropped).toBe(true);
  await load(page, small.view);
  await expect(items(page)).toHaveCount(2);
  await items(page).nth(0).locator("button").click();
  await expect(page.locator("#actions > li.gap")).toHaveText("log dropped");
  await expect(segments(page)).toHaveCount(0);
});

test("an entry over 1 MiB", async ({ page, surf }) => {
  test.setTimeout(60_000);
  const id = await surf.open();
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(1);
  // One go to an address of 60,000 bytes: its answer entry is over 1,048,576 bytes (the address
  // is written three times, each `<` as <), which the session's test
  // large_entry_one_go_makes_an_answer_over_1_mib_and_three_make_four_actions measures from the
  // detail; here the view draws that entry (story:page-step-holds-under-load).
  const address = surf.pages + "/first/a.html?" + "<".repeat(60_000);
  expect(await surf.agent(id, "go", address)).toBe(0);
  await expect(segments(page)).toHaveCount(2, { timeout: 30_000 });
  await expect(pageAddress(page)).toHaveCount(1, { timeout: 30_000 });
  await expect.poll(() => pageAddress(page).textContent(), { timeout: 30_000 }).toBe(address);
});

// story:view-live-means-stream-open: the word `live` says the page follows. It is written when a
// stream is open and the load its opening started is drawn, and not before.

test("the word is connecting until the stream is open", async ({ page, surf }) => {
  const id = await surf.open(surf.pages + "/first/a.html");
  const summary = (await surf.api("/api/sessions")).sessions.find((s) => s.id === id);
  const stream = await hold(page, /\/events$/, 1);
  await page.goto(surf.view);
  await stream.asked;
  await expect(items(page)).toHaveCount(1);
  await expect(items(page).nth(0).locator(".address")).toHaveText(summary.address);
  await expect(page.locator("#live-word"), "the first load is shown, the stream is not open").toHaveText("connecting");
  stream.release();
  await expectLive(page);
});

test("the word is live after the load of the opening is drawn", async ({ page, surf }) => {
  const id = await surf.open();
  const list = await hold(page, /\/api\/sessions$/, 2);
  await page.goto(surf.view);
  await list.asked;
  await expect(segments(page)).toHaveCount(1);
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await expect(segments(page), "an event of the open stream arrived and was drawn").toHaveCount(2);
  await expect(page.locator("#live-word"), "the load of the opening is not drawn yet").toHaveText("connecting");
  list.release();
  await expectLive(page);
});

test("after a reconnect the word is live when its load is drawn", async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await load(page, surf.piped);
  await expect(segments(page)).toHaveCount(2);
  await surf.cut();
  await expect(page.locator("#live-word")).toHaveText("reconnecting", { timeout: 5000 });
  const list = await hold(page, /\/api\/sessions$/, 1);
  await surf.mend();
  await list.asked;
  await expect(page.locator("#live-word"), "the load of the new opening is not drawn yet").toHaveText("reconnecting");
  list.release();
  await expectLive(page);
});

test("no render of the timeline follows live", async ({ page, surf, record }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  // One observer over the document: its records come in the order of the mutations, so a clear
  // of #actions counts when the record that wrote `live` into #live-word came before it.
  await page.addInitScript(() => {
    window.surfTestClearsAfterLive = 0;
    let live = false;
    new MutationObserver((records) => {
      for (const record of records) {
        if (record.target.id === "live-word") {
          if ([...record.addedNodes].some((node) => node.textContent === "live")) {
            live = true;
          }
        } else if (live && record.target.id === "actions" && record.removedNodes.length > 0) {
          window.surfTestClearsAfterLive += 1;
        }
      }
    }).observe(document, { childList: true, subtree: true });
  });
  await load(page, surf.view);
  await expect.poll(() => record.requestsOf("/api/sessions/" + id).length).toBeGreaterThanOrEqual(2);
  await record.requestsOf("/api/sessions/" + id)[1].body;
  await frames(page);
  expect(await page.evaluate(() => window.surfTestClearsAfterLive), "clears of #actions after live").toBe(0);
});

test("no word is written while the page does not follow", async ({ page, surf }) => {
  await surf.open();
  // Every word #live-word takes, from the text node each write adds (dom.js written()), and
  // every clear of #actions, in one list in the order of the mutation records.
  await page.addInitScript(() => {
    window.surfTestWritten = [];
    new MutationObserver((records) => {
      for (const record of records) {
        if (record.target.id === "live-word") {
          for (const node of record.addedNodes) {
            window.surfTestWritten.push("word " + node.textContent);
          }
        } else if (record.target.id === "actions" && record.removedNodes.length > 0) {
          window.surfTestWritten.push("clear");
        }
      }
    }).observe(document, { childList: true, subtree: true });
  });
  // The first stream opens, gives one keep-alive comment and ends; the next one is held.
  const stream = { asked: null, release: null };
  let arrived = null;
  stream.asked = new Promise((resolve) => {
    arrived = resolve;
  });
  const released = new Promise((resolve) => {
    stream.release = resolve;
  });
  let streams = 0;
  await page.route(/\/events$/, async (route) => {
    streams += 1;
    if (streams === 1) {
      await route.fulfill({ status: 200, headers: { "Content-Type": "text/event-stream" }, body: ": keep-alive\n\n" });
      return;
    }
    if (streams === 2) {
      arrived();
      await released;
    }
    await route.continue();
  });
  // The load of the first opening is held.
  const list = await hold(page, /\/api\/sessions$/, 2);
  const word = page.locator("#live-word");
  const written = () => page.evaluate(() => window.surfTestWritten.slice());
  await page.goto(surf.view);
  await list.asked;
  await stream.asked;
  await expect(word, "a keep-alive before the draw writes no word").toHaveText("reconnecting");
  expect(await written(), "a keep-alive before the draw writes no word").not.toContain("word live");
  const before = (await written()).length;
  list.release();
  await expect
    .poll(async () => (await written()).slice(before), { message: "the released load is drawn" })
    .toContain("clear");
  await expect(word, "a load whose stream closed writes no word").toHaveText("reconnecting");
  expect(await written(), "a load whose stream closed writes no word").not.toContain("word live");
  stream.release();
  await expectLive(page);
});
