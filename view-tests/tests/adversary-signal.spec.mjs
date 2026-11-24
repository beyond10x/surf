// Adversary case against story:session-process-hardening, decision 6: the page shows the signal a
// session ended by beside `died`. The fixture's `kill` sends SIGKILL from outside surfd, so the
// summary's `signal` is "SIGKILL" and the facts line of the selected session reads it.

import { test, expect, expectLive } from "./fixture.mjs";

const items = (page) => page.locator("#session-list > li");

test("a session killed from outside shows its signal beside died", async ({ page, surf }) => {
  const dying = await surf.open(surf.pages + "/first/a.html");
  await page.goto(surf.view);
  await expectLive(page);
  await expect(items(page)).toHaveCount(1);
  await surf.kill(dying);
  await expect(items(page).nth(0).locator(".state")).toHaveText("died");
  const summary = (await surf.api("/api/sessions")).sessions.find((s) => s.id === dying);
  expect(summary.signal).toBe("SIGKILL");
  await items(page).nth(0).locator("button").click();
  await expect(page.locator("#session-facts")).toContainText("died (SIGKILL)");
  await expect(page.locator("#session-facts *")).toHaveCount(0);
});
