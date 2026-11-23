// Cases 3 to 7 of story:view-controls-and-changes, each a test named after its case, each with
// its own surf-view-fixture (fixture.mjs) over the pages of crates/view-fixture/pages/forms/.
// The panel "Now" groups a page's controls under fields and its buttons under buttons; the panel
// "Changed by <action>" shows the changed member of the answer entry at the playhead. A password
// is shown as the length its FillRequested entry holds, never as its value.

import { test, expect } from "./fixture.mjs";

// The password of cases 3, 5 and 6, given on the standard input of surf-agent.
const SECRET = "hunter2-view";

// The label, the value and the text of crates/view-fixture/pages/forms/forged.html.
const FORGED = '"<script>alert(1)</script><img src=x onerror=alert(1)>';
// The same text as its Markdown line writes it, each "<" and ">" behind a backslash.
const FORGED_MARKDOWN = '"\\<script\\>alert(1)\\</script\\>\\<img src=x onerror=alert(1)\\>';

const segments = (page) => page.locator("#actions > li.segment");
const changes = (page) => page.locator("#changes li.change");

async function load(page, address) {
  await page.goto(address);
  await expect(page.locator("#live-word")).toHaveText("live");
}

// The playhead onto segment `at`.
async function playheadTo(page, at) {
  await segments(page).nth(at).locator("button.select").click();
  await expect(segments(page).nth(at)).toHaveAttribute("aria-current", "step");
}

// Every element of the group `group` of the panel "Now": [handle, kind, label, value, flags]
// for fields, [handle, label, form, method, action] for buttons.
async function group(page, name) {
  return page.locator("#now section.now-group").evaluateAll(
    (sections, name) =>
      sections
        .filter((section) => section.querySelector("h4").textContent === name)
        .flatMap((section) =>
          [...section.querySelectorAll("li.now-element")].map((node) => {
            const text = (selector) => {
              const found = node.querySelector(selector);
              return found ? found.textContent : null;
            };
            if (name === "buttons") {
              return [text(".handle"), text(".label"), text(".submits .form"), text(".submits .method"), text(".submits .action")];
            }
            const flags = [...node.querySelectorAll(".flag")].map((flag) => flag.textContent).join(" ");
            return [text(".handle"), text(".kind"), text(".label"), text(".value"), flags];
          }),
        ),
    name,
  );
}

async function changed(page) {
  return changes(page).evaluateAll((nodes) => nodes.map((node) => node.textContent));
}

// Nothing of the page holds `value`: its markup with every attribute, its text, its address.
async function holdsNowhere(page, value) {
  expect(await page.content()).not.toContain(value);
  expect(await page.evaluate(() => document.documentElement.textContent)).not.toContain(value);
  expect(page.url()).not.toContain(value);
}

test("controls case 3: the fields and buttons groups with states, and the password as its length 12, nowhere its value", async ({
  page,
  surf,
  record,
}) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/forms/login.html");
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(2);
  await expect.poll(() => group(page, "fields")).toEqual([
    ["f1", "text", "Name", '""', ""],
    ["f2", "password", "Password", "(empty)", "required"],
    ["f3", "checkbox", "Remember me", null, "checked"],
    ["f4", "select", "Role", '"Editor"', ""],
  ]);
  expect(await group(page, "buttons")).toEqual([["b1", "Sign in", "F1", "post", "/forms/answer.html"]]);
  expect(await surf.agentStdin(id, SECRET, "fill", "f2", "-")).toBe(0);
  await expect(segments(page)).toHaveCount(3);
  await expect.poll(() => group(page, "fields")).toEqual([
    ["f1", "text", "Name", '""', ""],
    ["f2", "password", "Password", "12 bytes", "required"],
    ["f3", "checkbox", "Remember me", null, "checked"],
    ["f4", "select", "Role", '"Editor"', ""],
  ]);
  await holdsNowhere(page, SECRET);
  const answered = record.requests.filter((request) => request.path.startsWith("/api/"));
  expect(answered.length).toBeGreaterThan(0);
  for (const request of answered) {
    expect(await request.body).not.toContain(SECRET);
  }
  expect(await surf.apiText("/api/sessions/" + id)).not.toContain(SECRET);
});

test('controls case 4: after fill f1 ada the changed panel shows f1 value "" -> "ada"', async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/forms/login.html");
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(2);
  expect(await surf.agent(id, "fill", "f1", "ada")).toBe(0);
  await expect(segments(page)).toHaveCount(3);
  await expect(page.locator("#changes-heading")).toHaveText("Changed by fill f1");
  await expect.poll(() => changed(page)).toEqual(['f1 value "" -> "ada"']);
});

test('controls case 5: after press b1 the changed panel shows the page change and + text "Wrong password"', async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/forms/login.html");
  await load(page, surf.view);
  expect(await surf.agentStdin(id, SECRET, "fill", "f2", "-")).toBe(0);
  expect(await surf.agent(id, "press", "b1")).toBe(0);
  await expect(segments(page)).toHaveCount(4);
  await expect(page.locator("#changes-heading")).toHaveText("Changed by press b1");
  await expect
    .poll(() => changed(page))
    .toEqual(['page "' + surf.pages + '/forms/login.html" -> "' + surf.pages + '/forms/answer.html"', '+ text "Wrong password"']);
  await holdsNowhere(page, SECRET);
});

test("controls case 6: with the playhead on a fill the changed panel shows that action's lines, not the latest", async ({ page, surf }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/forms/login.html");
  expect(await surf.agentStdin(id, SECRET, "fill", "f2", "-")).toBe(0);
  expect(await surf.agent(id, "fill", "f1", "ada")).toBe(0);
  expect(await surf.agent(id, "press", "b1")).toBe(0);
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(5);
  await expect(page.locator("#changes-heading")).toHaveText("Changed by press b1");
  await playheadTo(page, 3);
  await expect(page.locator("#changes-heading")).toHaveText("Changed by fill f1");
  await expect.poll(() => changed(page)).toEqual(['f1 value "" -> "ada"']);
  await playheadTo(page, 2);
  await expect(page.locator("#changes-heading")).toHaveText("Changed by fill f2");
  await expect.poll(() => changed(page)).toEqual(["f2 value (empty) -> 12 bytes"]);
  expect((await group(page, "fields"))[1]).toEqual(["f2", "password", "Password", "12 bytes", "required"]);
  await holdsNowhere(page, SECRET);
});

test("controls case 7: a label, a value and a text line with markup are text: no script, no img, no dialog", async ({ page, surf, record }) => {
  const id = await surf.open();
  await surf.agent(id, "go", surf.pages + "/forms/forged.html");
  await load(page, surf.view);
  await expect(segments(page)).toHaveCount(2);
  await expect.poll(() => group(page, "fields")).toEqual([["f1", "text", FORGED, '"' + FORGED + '"', ""]]);
  await expect(changes(page)).toHaveCount(3);
  expect((await changed(page))[1]).toBe('+ text "' + FORGED_MARKDOWN + '"');
  expect(await page.locator("#now script, #now img, #changes script, #changes img").count()).toBe(0);
  expect(await page.locator("img").count()).toBe(0);
  expect(await page.locator("script").count()).toBe(1);
  expect(record.dialogs).toEqual([]);
  expect(record.errors).toEqual([]);
  expect(record.violations).toEqual([]);
});
