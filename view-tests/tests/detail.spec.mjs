// The 6 cases of story:view-detail-panel, each a test named after it, each with its own
// surf-view-fixture (fixture.mjs) and its existing commands only. Every wait is on the thing
// itself; the deadline is the story's where it names one (2 s), otherwise the config's 10 s.

import { test, expect } from "./fixture.mjs";

// The story's query is '?"<script>alert(1)</script><img src=x onerror=alert(1)>'; the session's
// fetch fails a query with a space as Unreachable (measured on 2026-11-21), so there is no
// ResponseReceived. The same markup without a space: an img whose src would be fetched.
const HOSTILE = '?"<script>alert(1)</script><img/src=x/onerror=alert(1)>';

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

const segments = (page) => page.locator("#actions > li.segment");
const selection = (page) => page.locator("#selection");
const rows = (page) => page.locator("#requests > li.request");
const latest = (page) => page.locator("#latest");

// The button of the mark named `short` (the event's short name) in segment `at`.
const markButton = (page, at, short) =>
  segments(page).nth(at).locator("li.mark").filter({ hasText: new RegExp("^" + short + "$") }).locator("button");

async function load(page, address) {
  await page.goto(address);
  await expect(page.locator("#live-word")).toHaveText("live");
}

// The index of the segment that carries the playhead, or -1.
async function playheadAt(page) {
  return segments(page).evaluateAll((nodes) => nodes.findIndex((node) => node.getAttribute("aria-current") === "step"));
}

// The selected event's fields as the panel shows them: [name, value] per dt/dd pair of dl.fields.
async function shownFields(page) {
  return selection(page)
    .locator("dl.fields")
    .evaluate((list) => {
      const names = [...list.querySelectorAll("dt")].map((node) => node.textContent);
      const values = [...list.querySelectorAll("dd")].map((node) => node.textContent);
      return names.map((name, at) => [name, values[at]]);
    });
}

// The byte length of a page the fixture serves, fetched from Node.
async function length(url) {
  const answer = await fetch(url);
  expect(answer.status).toBe(200);
  return (await answer.arrayBuffer()).byteLength;
}

function eventOf(action, name) {
  const found = action.events.filter((event) => event.name === name);
  expect(found.length, name + " in the action").toBe(1);
  return found[0];
}

test("selected action", async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  const detail = await surf.api("/api/sessions/" + id);
  const second = actionsOf(detail)[1];
  expect(second.answer).not.toBeNull();
  const between = detail.entries.filter(
    (entry) => entry.kind === "event" && entry.number > second.action.number && entry.number < second.answer.number,
  ).length;
  expect(between).toBeGreaterThan(0);
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(3);
  await segments(page).nth(1).locator("button.select").click();
  await expect.poll(() => playheadAt(page)).toBe(1);
  await expect(selection(page).locator(".sequence")).toHaveText("2");
  await expect(selection(page).locator(".command")).toHaveText("go");
  expect(await selection(page).locator(".argument").textContent()).toBe(surf.pages + "/first/a.html");
  const kept = await page.evaluate((ms) => new Date(ms).toLocaleTimeString([], { hour12: false }), second.action.kept_at);
  await expect(selection(page).locator(".kept")).toHaveText(kept);
  await expect(selection(page).locator(".events")).toHaveText(String(between));
});

test("selected event", async ({ page, surf, record }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  const response = eventOf(actionsOf(await surf.api("/api/sessions/" + id))[1], "surf.net.ResponseReceived");
  const size = await length(surf.pages + "/first/a.html");
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(3);
  await markButton(page, 1, "ResponseReceived").click();
  await expect(selection(page).locator(".name")).toHaveText("surf.net.ResponseReceived");
  const fields = await shownFields(page);
  expect(fields.map(([name]) => name), "every field in the entry's order").toEqual(Object.keys(response.fields));
  const shown = new Map(fields);
  expect(shown.get("request_id")).toBe(response.fields.request_id);
  expect(shown.get("url")).toBe(surf.pages + "/first/a.html");
  expect(shown.get("status")).toBe("200");
  expect(shown.get("media_type")).toBe("text/html");
  expect(shown.get("body")).toBe(size + " bytes");
  await expect(markButton(page, 1, "ResponseReceived")).toHaveAttribute("aria-pressed", "true");
  await expect(latest(page)).toHaveAttribute("aria-pressed", "false");
  expect(await playheadAt(page)).toBe(1);
  // A null field value reads "no value": the open's ActionRecorded has no url.
  const recorded = eventOf(actionsOf(await surf.api("/api/sessions/" + id))[0], "surf.agent.ActionRecorded");
  expect(recorded.fields.url).toBeNull();
  await markButton(page, 0, "ActionRecorded").click();
  await expect(selection(page).locator(".name")).toHaveText("surf.agent.ActionRecorded");
  expect(new Map(await shownFields(page)).get("url")).toBe("no value");
  await expect(markButton(page, 1, "ResponseReceived")).toHaveAttribute("aria-pressed", "false");
  expect(await playheadAt(page)).toBe(0);
  expect(record.errors).toEqual([]);
});

test("requests list", async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  await surf.agent(id, "go", surf.unreachable);
  const sizes = [await length(surf.pages + "/first/a.html"), await length(surf.pages + "/first/b.html")];
  expect(sizes).toEqual([209, 210]);
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(4);
  await expect(rows(page)).toHaveCount(3);
  const addresses = [surf.pages + "/first/a.html", surf.pages + "/first/b.html", surf.unreachable];
  for (const [at, address] of addresses.entries()) {
    expect(await rows(page).nth(at).locator(".address").textContent()).toBe(address);
  }
  for (const [at, size] of sizes.entries()) {
    const row = rows(page).nth(at);
    await expect(row.locator(".status")).toHaveText("200");
    await expect(row.locator(".media-type")).toHaveText("text/html");
    await expect(row.locator(".size")).toHaveText(size + " bytes");
    await expect(row.locator(".reason")).toHaveCount(0);
  }
  const failed = rows(page).nth(2);
  await expect(failed.locator(".reason")).toHaveText("Unreachable");
  await expect(failed.locator(".status")).toHaveCount(0);
  await expect(failed.locator(".pending")).toHaveCount(0);
});

test("a go adds a row live", async ({ page, surf, record }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  await surf.agent(id, "go", surf.unreachable);
  await load(page, surf.view);
  await expect(rows(page)).toHaveCount(3);
  const roots = record.requestsOf("/").length;
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  // The bound of 2 s is the sentence's "a fourth row within 2 s", named beside it.
  await expect(rows(page)).toHaveCount(4, { timeout: 2000 }); // .engineering/planning/story/view-detail-panel.md:156-157
  const row = rows(page).nth(3);
  await expect(row.locator(".status")).toHaveText("200");
  await expect(row.locator(".size")).toHaveText("210 bytes");
  expect(await row.locator(".address").textContent()).toBe(surf.pages + "/first/b.html");
  expect(record.requestsOf("/").length).toBe(roots);
});

test("event in the address", async ({ page, surf, record }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/first/a.html");
  await surf.agent(id, "go", surf.pages + "/first/b.html");
  const response = eventOf(actionsOf(await surf.api("/api/sessions/" + id))[1], "surf.net.ResponseReceived");
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(3);
  await markButton(page, 1, "ResponseReceived").click();
  const fragment = "#session=" + id + "&at=2&event=" + response.number;
  await expect.poll(() => page.evaluate(() => location.hash)).toBe(fragment);
  await page.reload();
  await expect(page.locator("#live-word")).toHaveText("live");
  await expect(segments(page)).toHaveCount(3);
  await expect(selection(page).locator(".name")).toHaveText("surf.net.ResponseReceived");
  expect(new Map(await shownFields(page)).get("request_id")).toBe(response.fields.request_id);
  await expect.poll(() => playheadAt(page)).toBe(1);
  await expect(latest(page)).toHaveAttribute("aria-pressed", "false");
  await expect(markButton(page, 1, "ResponseReceived")).toHaveAttribute("aria-pressed", "true");
  expect(await page.evaluate(() => location.hash)).toBe(fragment);
  expect(await page.evaluate(() => location.href)).not.toContain(surf.token);
  for (const address of record.addresses.slice(1)) {
    expect(address).not.toContain(surf.token);
  }
  await segments(page).nth(2).locator("button.select").click();
  await expect.poll(() => page.evaluate(() => location.hash)).toBe("#session=" + id + "&at=3");
  await expect(markButton(page, 1, "ResponseReceived")).toHaveAttribute("aria-pressed", "false");
  await expect(selection(page).locator(".name")).toHaveCount(0);
});

test("hostile address is text in the panel", async ({ page, surf, record }) => {
  const id = await surf.open();
  const address = surf.pages + "/first/a.html" + HOSTILE;
  await surf.agent(id, "go", address);
  const go = actionsOf(await surf.api("/api/sessions/" + id))[1];
  const started = eventOf(go, "surf.net.RequestStarted");
  const response = eventOf(go, "surf.net.ResponseReceived");
  expect(go.action.argument).toContain("<script>");
  expect(response.fields.url).toContain("<script>");
  expect(started.fields.url).toContain("<script>");
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(2);
  await expect(selection(page).locator(".argument")).toHaveCount(1);
  expect(await selection(page).locator(".argument").textContent()).toBe(go.action.argument);
  await expect(rows(page)).toHaveCount(1);
  expect(await rows(page).nth(0).locator(".address").textContent()).toBe(started.fields.url);
  await markButton(page, 1, "ResponseReceived").click();
  await expect(selection(page).locator(".name")).toHaveText("surf.net.ResponseReceived");
  expect(new Map(await shownFields(page)).get("url")).toBe(response.fields.url);
  expect(
    await page.evaluate(() => ({
      script: document.querySelectorAll("script").length,
      img: document.querySelectorAll("img").length,
    })),
  ).toEqual({ script: 1, img: 0 });
  expect(record.dialogs).toEqual([]);
  expect(record.requests.filter((request) => request.path === "/x" || request.path.startsWith("/x/"))).toEqual([]);
  expect(record.violations).toEqual([]);
  expect(record.errors).toEqual([]);
});
