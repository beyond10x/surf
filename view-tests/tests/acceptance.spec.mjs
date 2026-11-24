// The page part of the live view's acceptance case (epic:live-view, story:view-acceptance):
// cases 4 to 7, each a test named after its sentence. Each test drives the page against its own
// surf-view-fixture (fixture.mjs: a surfd the test starts) with two sessions open, over the pages
// of crates/view-fixture/pages/acceptance/. The HTTP part is
// crates/view-fixture/tests/acceptance.rs. No fixed sleep: every wait is on an element, a count
// or an attribute, with the runner's timeout.

import { test, expect, expectLive } from "./fixture.mjs";

const TITLE = "Start <script>alert(1)</script>";

const items = (page) => page.locator("#session-list > li");
const segments = (page) => page.locator("#actions > li.segment");
const elements = (page) => page.locator("#now li.now-element");
const latest = (page) => page.locator("#latest");
const pagePart = (page, name) =>
  page.locator("#page-line .part").filter({ has: page.locator("dt", { hasText: new RegExp("^" + name + "$") }) }).locator("dd");

async function load(page, address) {
  await page.goto(address);
  await expectLive(page);
}

// The index of the segment that carries the playhead, or -1.
async function playheadAt(page) {
  return segments(page).evaluateAll((nodes) => nodes.findIndex((node) => node.getAttribute("aria-current") === "step"));
}

// Two sessions; the second is selected by a click, as a person would.
async function twoSessions(page, surf) {
  const first = await surf.open();
  const second = await surf.open();
  await load(page, surf.view);
  await expect(items(page)).toHaveCount(2);
  await items(page).nth(1).locator("button").click();
  await expect(items(page).nth(1)).toHaveAttribute("aria-current", "true");
  return [first, second];
}

test("page lists both sessions", async ({ page, surf }) => {
  const [first, second] = await twoSessions(page, surf);
  const listed = await surf.api("/api/sessions");
  expect(listed.sessions.map((summary) => summary.id)).toEqual([first, second]);
  await expect(items(page).locator(".state")).toHaveText(["open", "open"]);
  await expect(page.locator("#session-facts")).toContainText("session " + second);
});

test("page line and three links after go", async ({ page, surf }) => {
  const [, second] = await twoSessions(page, surf);
  const start = surf.pages + "/acceptance/start.html";
  await surf.agent(second, "go", start);
  await expect(segments(page)).toHaveCount(2);
  await expect(pagePart(page, "status")).toHaveText("200");
  await expect(pagePart(page, "media type")).toHaveText("text/html");
  await expect(pagePart(page, "address")).toHaveText(start);
  await expect(elements(page)).toHaveCount(3);
  await expect(elements(page).locator(".handle")).toHaveText(["l1", "l2", "l3"]);
  await expect(elements(page).locator(".label")).toHaveText(["First page", "Second page", "Third page"]);
  await expect(elements(page).locator(".target")).toHaveText([
    "/acceptance/one.html",
    "/acceptance/two.html",
    "/acceptance/three.html",
  ]);
  await expect(page.locator("#now .now-group h4")).toHaveText(["links"]);
});

test("latest marker moves on second go", async ({ page, surf }) => {
  const [, second] = await twoSessions(page, surf);
  await surf.agent(second, "go", surf.pages + "/acceptance/start.html");
  await expect(segments(page)).toHaveCount(2);
  await expect.poll(() => playheadAt(page)).toBe(1);
  await expect(latest(page)).toHaveAttribute("aria-pressed", "true");
  const one = surf.pages + "/acceptance/one.html";
  await surf.agent(second, "go", one);
  await expect(segments(page)).toHaveCount(3);
  await expect.poll(() => playheadAt(page)).toBe(2);
  await expect(latest(page)).toHaveAttribute("aria-pressed", "true");
  await expect(pagePart(page, "address")).toHaveText(one);
  await expect(elements(page).locator(".label")).toHaveText(["Back to the start"]);
});

test("script in title is shown as text", async ({ page, surf, record }) => {
  const [, second] = await twoSessions(page, surf);
  const scripts = await page.locator("script").count();
  await surf.agent(second, "go", surf.pages + "/acceptance/start.html");
  const title = items(page).nth(1).locator(".title");
  await expect(title).toHaveCount(1);
  expect(await title.textContent()).toBe(TITLE);
  await expect(pagePart(page, "title")).toHaveCount(1);
  expect(await pagePart(page, "title").textContent()).toBe(TITLE);
  const summary = (await surf.api("/api/sessions")).sessions.find((held) => held.id === second);
  expect(summary.title, "the exact title, no escape of the link left (D26)").toBe(TITLE);
  expect(await page.locator("script").count()).toBe(scripts);
  expect(record.dialogs).toEqual([]);
  expect(record.errors).toEqual([]);
});
