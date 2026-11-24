// Cases 2 to 6 of story:view-now-panel, each a test named after its case, each with its own
// surf-view-fixture (fixture.mjs) over the pages of crates/view-fixture/pages/now/. The panel
// "Now" shows the now block of the answer entry of the action at the playhead.

import { test, expect, expectLive } from "./fixture.mjs";

// The three values of cases 5 and 6, together: the title of now/forged.html, and its one
// link's label; its target is "javascript:" and the same three in quotes.
const FORGED = '"<script>alert(1)</script><img src=x onerror=alert(1)>';

const segments = (page) => page.locator("#actions > li.segment");
const panel = (page) => page.locator("#now");
const elements = (page) => page.locator("#now li.now-element");

async function load(page, address) {
  await page.goto(address);
  await expectLive(page);
}

async function texts(locator) {
  return locator.evaluateAll((nodes) => nodes.map((node) => node.textContent));
}

// The index of the segment that carries the playhead, or -1.
async function playheadAt(page) {
  return segments(page).evaluateAll((nodes) => nodes.findIndex((node) => node.getAttribute("aria-current") === "step"));
}

// Every element the panel lists: [group, handle, visibility, label, target] each.
async function listed(page) {
  return panel(page)
    .locator("li.now-element")
    .evaluateAll((nodes) =>
      nodes.map((node) => [
        node.closest("section.now-group").querySelector("h4").textContent,
        node.querySelector(".handle").textContent,
        node.querySelector(".visibility").textContent,
        node.querySelector(".label").textContent,
        node.querySelector(".target").textContent,
      ]),
    );
}

test("now case 2: the three links under links, l1 to l3 in document order", async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/now/links.html");
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(2);
  await expect(elements(page)).toHaveCount(3);
  expect(await listed(page)).toEqual([
    ["links", "l1", "?", "Next page", "/now/next.html"],
    ["links", "l2", "hid", "Members", "/now/members.html"],
    ["links", "l3", "?", "Broken", "http://["],
  ]);
  await expect(panel(page).locator(".now-count")).toHaveText("3 of 3");
  await expect(elements(page).nth(2).locator(".unresolved")).toHaveText("unresolved");
  await expect(elements(page).nth(0).locator(".unresolved")).toHaveCount(0);
});

test("now case 3: the hidden link is marked hidden and the visibility note is shown", async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/now/links.html");
  await load(page, surf.view);
  await expect(elements(page)).toHaveCount(3);
  const hidden = elements(page).filter({ has: page.locator(".handle", { hasText: /^l2$/ }) });
  await expect(hidden).toHaveClass(/\bis-hidden\b/);
  await expect(hidden.locator(".visibility")).toHaveText("hid");
  await expect(hidden.locator(".visibility")).toHaveAttribute("title", "hidden attribute");
  for (const at of [0, 2]) {
    await expect(elements(page).nth(at)).not.toHaveClass(/\bis-hidden\b/);
    await expect(elements(page).nth(at).locator(".visibility")).toHaveText("?");
  }
  await expect(panel(page).locator(".now-note")).toHaveText("visibility: hidden attribute only");
});

test("now case 4: after follow l1 a segment follow l1 and the next page's links; the earlier action keeps its own", async ({
  page,
  surf,
}) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/now/links.html");
  await load(page, surf.view);
  await expect(elements(page)).toHaveCount(3);
  expect(await surf.agent(id, "follow", "l1")).toBe(0);
  await expect(segments(page)).toHaveCount(3);
  const followed = segments(page).nth(2).locator("button.select");
  await expect(followed.locator(".command")).toHaveText("follow");
  await expect(followed.locator(".handle")).toHaveText("l1");
  await expect.poll(() => listed(page)).toEqual([
    ["links", "l1", "?", "Back", "/now/links.html"],
    ["links", "l2", "?", "Elsewhere", "http://other.example/x?y#z"],
  ]);
  await segments(page).nth(1).locator("button.select").click();
  await expect.poll(() => playheadAt(page)).toBe(1);
  await expect.poll(() => listed(page)).toEqual([
    ["links", "l1", "?", "Next page", "/now/next.html"],
    ["links", "l2", "hid", "Members", "/now/members.html"],
    ["links", "l3", "?", "Broken", "http://["],
  ]);
  // The open's answer has no page and no now block.
  await segments(page).nth(0).locator("button.select").click();
  await expect.poll(() => playheadAt(page)).toBe(0);
  await expect(elements(page)).toHaveCount(0);
  await expect(panel(page).locator(".now-empty")).toHaveText("nothing to act on");
});

test("now case 5: a title with markup is text in the session list and on the page line", async ({ page, surf, record }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/now/forged.html");
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(2);
  const title = page.locator("#session-list .title");
  await expect(title).toHaveCount(1);
  expect(await title.textContent()).toBe(FORGED);
  const line = page.locator("#page-line .part").filter({ has: page.locator("dt", { hasText: /^title$/ }) }).locator("dd");
  await expect(line).toHaveCount(1);
  expect(await line.textContent()).toBe(FORGED);
  expect(await page.locator("#session-heading").textContent()).toBe(FORGED);
  expect(await page.locator("#session-list script, #session-list img, #page-line script, #page-line img").count()).toBe(0);
  expect(await page.locator("img").count()).toBe(0);
  expect(await page.locator("script").count()).toBe(1);
  expect(record.dialogs).toEqual([]);
  expect(record.errors).toEqual([]);
  expect(record.violations).toEqual([]);
});

test("now case 6: a label and a target with markup are text in the panel", async ({ page, surf, record }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/now/forged.html");
  await load(page, surf.view);
  await expect(elements(page)).toHaveCount(1);
  expect(await texts(elements(page).locator(".label"))).toEqual([FORGED]);
  expect(await texts(elements(page).locator(".target"))).toEqual(["javascript:" + FORGED + '"']);
  expect(await panel(page).locator("script, img, a, [href], [src]").count()).toBe(0);
  expect(await page.locator("img").count()).toBe(0);
  expect(await page.locator("script").count()).toBe(1);
  await elements(page).nth(0).click();
  expect(await page.locator("img").count()).toBe(0);
  expect(record.dialogs).toEqual([]);
  expect(record.errors).toEqual([]);
  expect(record.violations).toEqual([]);
});
