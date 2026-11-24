// Adversary case against story:page-test-step-within-budget, statement 7, reworked to the clock
// (D318, D321): the wait of "adversary: a refused token on the stream stops the page" went from
// 12,000 ms to 2,000 ms, and a page that asks /events again after LAST_DELAY (5,000 ms, the
// longest delay of crates/view/web/stream.js) passed it. The page here is the view with one
// change, served in place of stream.js by a route (no rebuild, nothing of crates/view changed):
// after a refusal it asks /events once more, after LAST_DELAY. The steps below are the shipped
// test's (adversary-page.spec.mjs), with its clock and its mark: on this page they see the
// request, so the shipped test fails on it.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { test, expect, expectLive } from "./fixture.mjs";

const REFUSED = "The view refused the token. Open the address the program printed, with its #token.";
const AFTER = "/after-the-delays";
const items = (page) => page.locator("#session-list > li");
const segments = (page) => page.locator("#actions > li.segment");

const here = path.dirname(fileURLToPath(import.meta.url));
const source = fs.readFileSync(path.join(here, "../../crates/view/web/stream.js"), "utf8");
const STOP = "          stopped = true;\n          handlers.refused(error);\n          return;\n";
const AGAIN =
  "          stopped = true;\n          handlers.refused(error);\n" +
  '          setTimeout(() => { fetch("/events", { headers: { Authorization: "Bearer " + token, "Last-Event-ID": String(last) }, cache: "no-store", credentials: "omit" }).catch(() => {}); }, LAST_DELAY);\n' +
  "          return;\n";

async function load(page, address) {
  await page.goto(address);
  await expectLive(page);
}

test("adversary-budget: a page that asks /events again 5 s after a refusal is seen by the refused-token test's steps", async ({ page, surf, record }) => {
  expect(source.includes(STOP), "stream.js no longer holds the refusal branch this plant replaces").toBe(true);
  const planted = source.replace(STOP, AGAIN);
  await page.route(/\/stream\.js(\?.*)?$/, (route) =>
    route.fulfill({ status: 200, contentType: "text/javascript; charset=utf-8", body: planted }),
  );
  // From here the shipped test's steps (adversary-page.spec.mjs, "a refused token on the stream
  // stops the page").
  await page.route(/\/after-the-delays$/, (route) => route.fulfill({ status: 204, body: "" }));
  await page.clock.install();
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
  await expect(page.locator("#message")).toHaveText(REFUSED);
  await surf.mend();
  const refusedAt = record.requests.length;
  await items(page).nth(0).locator("button").click();
  await page.clock.runFor(12_500);
  await page.evaluate((after) => fetch(after).then(() => {}, () => {}), AFTER);
  // The shipped test asserts [AFTER]; on this page the request after LAST_DELAY is there first.
  expect(record.requests.slice(refusedAt).map((request) => request.path)).toEqual(["/events", AFTER]);
});
