// The adversary's cases against story:view-now-panel, each with its own surf-view-fixture
// (fixture.mjs) over the pages of crates/view-fixture/pages/adversary-now/ and now/. Each waits
// on a poll of the page (expect), never on a fixed time.

import { test, expect } from "./fixture.mjs";

// U+202E RIGHT-TO-LEFT OVERRIDE, a bidirectional control.
const RLO = String.fromCodePoint(0x202e);

const segments = (page) => page.locator("#actions > li.segment");
const panel = (page) => page.locator("#now");
const elements = (page) => page.locator("#now li.now-element");

async function load(page, address) {
  await page.goto(address);
  await expect(page.locator("#live-word")).toHaveText("live");
}

// The class of each child of each listed element, in order: where the cut marks are.
async function shapes(page) {
  return elements(page).evaluateAll((nodes) => nodes.map((node) => [...node.children].map((child) => child.className)));
}

// The characters of `wanted` in `node`'s text, in the order they appear on the screen, left to
// right.
function onScreen(node, wanted) {
  const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
  const found = [];
  for (let text = walker.nextNode(); text; text = walker.nextNode()) {
    for (let at = 0; at < text.data.length; at += 1) {
      if (wanted.includes(text.data[at])) {
        const range = document.createRange();
        range.setStart(text, at);
        range.setEnd(text, at + 1);
        found.push([text.data[at], range.getBoundingClientRect().left]);
      }
    }
  }
  return found
    .sort((a, b) => a[1] - b[1])
    .map((pair) => pair[0])
    .join("");
}

test("adversary now: an answer without a now block after one with links shows nothing to act on", async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/now/links.html");
  await surf.agent(id, "go", surf.pages + "/adversary-now/plain.html");
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(3);
  await expect(panel(page).locator(".now-empty")).toHaveText("nothing to act on");
  await expect(elements(page)).toHaveCount(0);
  await segments(page).nth(1).locator("button.select").click();
  await expect(elements(page)).toHaveCount(3);
  await page.locator("#latest").click();
  await expect(panel(page).locator(".now-empty")).toHaveText("nothing to act on");
  await expect(elements(page)).toHaveCount(0);
});

test("adversary now: a cut label and a cut target each carry their own cut mark", async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/adversary-now/cut.html");
  await load(page, surf.view);
  await expect(elements(page)).toHaveCount(2);
  expect(await shapes(page)).toEqual([
    ["handle", "visibility", "label", "cut", "arrow", "target"],
    ["handle", "visibility", "label", "arrow", "target", "cut"],
  ]);
  expect(await elements(page).nth(0).locator(".label").textContent()).toBe("L".repeat(60));
  expect(await elements(page).nth(0).locator(".target").textContent()).toBe("/adversary-now/a.html");
  expect(await elements(page).nth(1).locator(".label").textContent()).toBe("Long target");
  const target = await elements(page).nth(1).locator(".target").textContent();
  expect(target.length).toBe(1024);
  expect(target).toBe("/adversary-now/" + "t".repeat(1024 - "/adversary-now/".length));
});

test("adversary now: a bidirectional control in a label or a target does not reorder what the person reads", async ({
  page,
  surf,
}) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/adversary-now/bidi.html");
  await load(page, surf.view);
  await expect(elements(page)).toHaveCount(1);
  const held = elements(page).nth(0);
  // The control is shown as a mark that names it (crates/view/web/dom.js), not written.
  expect(await held.locator(".label").textContent()).toBe("go U+202Eabc");
  expect(await held.locator(".target").textContent()).toBe("http://[U+202Exyz");
  expect(await held.locator(".bidi-mark").allTextContents()).toEqual(["U+202E", "U+202E"]);
  // Read left to right, the characters must come in the order of the value.
  expect({
    label: await held.locator(".label").evaluate(onScreen, "abc"),
    target: await held.locator(".target").evaluate(onScreen, "xyz"),
  }).toEqual({ label: "abc", target: "xyz" });
});

test("adversary now: markup in every member of a forged stream element is text", async ({ page, surf, record }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/now/links.html");
  const forged = [
    {
      handle: "<img src=x onerror=alert(1)>",
      kind: "<script>alert(2)</script>",
      visibility: '"><img src=x onerror=alert(3)>',
      label: null,
      label_cut: null,
      target: null,
      target_cut: null,
      unresolved: null,
      text: "</li><img src=x onerror=alert(4)>&lt;'\"",
    },
    {
      handle: "l2",
      kind: "link",
      visibility: "hid",
      label: "'\"><svg onload=alert(5)>",
      label_cut: false,
      target: " \tjava\tscript:alert(6)",
      target_cut: false,
      unresolved: false,
      text: null,
    },
  ];
  await page.route("**/api/sessions/" + id, async (route) => {
    const response = await route.fetch();
    const detail = await response.json();
    for (const entry of detail.entries) {
      if (entry.kind === "answer" && entry.now) {
        entry.now.elements = forged;
        entry.now.shown = 2;
        entry.now.total = 2;
      }
    }
    await route.fulfill({ response, json: detail });
  });
  await load(page, surf.view);
  await expect(elements(page)).toHaveCount(2);
  const first = elements(page).nth(0);
  expect(await first.locator(".handle").textContent()).toBe(forged[0].handle);
  expect(await first.locator(".visibility").textContent()).toBe(forged[0].visibility);
  expect(await first.locator(".text").textContent()).toBe(forged[0].text);
  expect(await panel(page).locator("h4").first().textContent()).toBe(forged[0].kind);
  const second = elements(page).nth(1);
  expect(await second.locator(".label").textContent()).toBe(forged[1].label);
  expect(await second.locator(".target").textContent()).toBe(forged[1].target);
  expect(await panel(page).locator("script, img, svg, a, [href], [src], [onerror], [onload]").count()).toBe(0);
  await second.click();
  expect(await page.locator("img, svg").count()).toBe(0);
  expect(await page.locator("script").count()).toBe(1);
  expect(record.dialogs).toEqual([]);
  expect(record.errors).toEqual([]);
  expect(record.violations).toEqual([]);
});

// The cases of the fix: the mark is made in dom.js, so a title and an address show it as a label
// does; a format character that is not a bidirectional control stays as it is.

test("adversary now: a bidirectional control in a title and in an address shows as a mark", async ({
  page,
  surf,
  record,
}) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/adversary-now/title.html");
  // No address a session reaches holds the control (the URL parser encodes it), so the answer's
  // address is forged on its way to the page.
  const forged = "http://127.0.0.1:9/" + RLO + "xyz";
  await page.route("**/api/sessions/" + id, async (route) => {
    const response = await route.fetch();
    const detail = await response.json();
    for (const entry of detail.entries) {
      if (entry.kind === "answer" && entry.page && entry.page.kind === "loaded") {
        entry.page.address = forged;
      }
    }
    await route.fulfill({ response, json: detail });
  });
  await load(page, surf.view);
  const part = (name) =>
    page
      .locator("#page-line .part")
      .filter({ has: page.locator("dt", { hasText: new RegExp("^" + name + "$") }) })
      .locator("dd");
  await expect(part("title")).toHaveCount(1);
  const title = page.locator("#session-list .title");
  await expect(title).toHaveCount(1);
  for (const node of [part("title"), title, page.locator("#session-heading")]) {
    expect(await node.textContent()).toBe("Title U+202Eabc");
    expect(await node.locator(".bidi-mark").allTextContents()).toEqual(["U+202E"]);
  }
  expect(await part("address").textContent()).toBe("http://127.0.0.1:9/U+202Exyz");
  expect(await part("address").locator(".bidi-mark").allTextContents()).toEqual(["U+202E"]);
  expect({
    title: await part("title").evaluate(onScreen, "abc"),
    address: await part("address").evaluate(onScreen, "xyz"),
  }).toEqual({ title: "abc", address: "xyz" });
  const held = await page.locator("body").evaluate((body) => body.textContent.includes(String.fromCodePoint(0x202e)));
  expect(held).toBe(false);
  expect(record.errors).toEqual([]);
  expect(record.violations).toEqual([]);
  // A later fetch of the page may reach the route after the fixture stopped.
  await page.unrouteAll({ behavior: "ignoreErrors" });
});

test("adversary now: a right-to-left mark in a label stays as it is", async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/adversary-now/mark.html");
  await load(page, surf.view);
  await expect(elements(page)).toHaveCount(1);
  const label = elements(page).nth(0).locator(".label");
  expect(await label.textContent()).toBe("go " + String.fromCodePoint(0x200f) + "abc");
  expect(await panel(page).locator(".bidi-mark").count()).toBe(0);
});

test("adversary now: a bidirectional control in an event field is named in the mark's title", async ({
  page,
  surf,
  record,
}) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/now/links.html");
  // A session's URL is percent-encoded, so the field is forged on its way to the page.
  await page.route("**/api/sessions/" + id, async (route) => {
    const response = await route.fetch();
    const detail = await response.json();
    for (const entry of detail.entries) {
      if (entry.kind === "event" && entry.name === "surf.agent.NavigationRequested") {
        entry.fields.url = "http://127.0.0.1:9/" + RLO + "xyz";
      }
    }
    await route.fulfill({ response, json: detail });
  });
  await load(page, surf.view);
  const mark = segments(page).locator("li.mark").filter({ hasText: /^NavigationRequested$/ });
  await expect(mark).toHaveCount(1);
  const title = await mark.getAttribute("title");
  expect(title).toContain("\nurl http://127.0.0.1:9/U+202Exyz");
  expect(title.includes(RLO)).toBe(false);
  expect(record.errors).toEqual([]);
  expect(record.violations).toEqual([]);
  // A later fetch of the page may reach the route after the fixture stopped.
  await page.unrouteAll({ behavior: "ignoreErrors" });
});
