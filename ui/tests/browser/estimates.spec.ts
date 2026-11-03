import { test, expect } from "./tool-host";
import { editSampleLabel, openOptionalSettings, prepareSampleInput } from "./sample-input";
import { readFile, writeFile, readdir } from "node:fs/promises";
import { resolve } from "node:path";
import type { Page } from "@playwright/test";
test.use({hostScenario:"estimates"});
async function paidDraft(page: Page, label = "ledger") {
  await page.getByRole("button",{name:/^Action /}).click();
  await page.getByRole("button",{name:/^Price a scripted call/}).click();
  await prepareSampleInput(page, label);
  await page.getByRole("textbox",{name:"Spending cap (USD)",exact:true}).fill("0.5");
}
const cost = (page: Page) => page.getByRole("region",{name:"Cost",exact:true});
const estimate = (page: Page) => page.getByRole("button",{name:"Estimate cost",exact:true});

test("paid draft estimates exact inputs without attempts and edits discard the quote", async ({page, host}, testInfo) => {
  let attempts = 0;
  const preparations: string[] = [];
  page.on("request", request => {
    const path = new URL(request.url()).pathname;
    if (path === "/api/attempts" && request.method() === "POST") attempts++;
    if (path === "/api/preparations" && request.method() === "POST") preparations.push(request.postDataJSON().id);
  });
  await host.open(page); await paidDraft(page);
  await expect(page.getByLabel("Max spend (USD)",{exact:true})).toHaveCount(0);
  expect(preparations).toHaveLength(0);
  await estimate(page).click();
  await expect(cost(page)).toContainText("Estimate within cap.");
  await expect(cost(page)).toContainText("$0.12");
  await cost(page).getByRole("heading").scrollIntoViewIfNeeded();
  await page.screenshot({path:testInfo.outputPath("live-cost.png")});
  await expect(page.getByRole("button",{name:"Run",exact:true})).toBeEnabled();
  const recorded = (await readFile(resolve(host.root,"../tool/estimates.ndjson"),"utf8")).trim().split("\n").map(line => JSON.parse(line));
  expect(recorded).toHaveLength(1);
  expect(recorded[0].arguments.arguments.max_spend_usd).toBe(0.5);
  expect(recorded[0].arguments.arguments.label).toBe("ledger");
  await editSampleLabel(page,"Changed after estimate");
  await expect(cost(page)).toContainText("Estimate again for the current inputs and session.");
  await expect(cost(page)).not.toContainText("Worst case");
  await expect(page.getByRole("button",{name:"Run",exact:true})).toBeDisabled();
  await expect.poll(async () => (await readFile(resolve(host.root,"../tool/estimates.ndjson"),"utf8")).trim().split("\n").length).toBe(1);
  await estimate(page).click();
  await expect(cost(page)).toContainText("Estimate within cap.");
  expect(preparations).toHaveLength(2);
  expect(preparations[0]).not.toBe(preparations[1]);
  await page.getByRole("textbox",{name:"Spending cap (USD)",exact:true}).fill("0.6");
  await expect(cost(page)).not.toContainText("Worst case");
  await expect(page.getByRole("button",{name:"Run",exact:true})).toBeDisabled();
  expect(preparations).toHaveLength(2);
  await page.getByRole("button",{name:"History",exact:true}).click();
  await expect(page.getByRole("dialog")).toContainText("No saved attempts");
  expect(attempts).toBe(0);
});

test("a late estimate after an edit is discarded and never restores old cost", async ({page, host}) => {
  await host.open(page); await paidDraft(page,"hold");
  await estimate(page).click();
  await expect.poll(async () => readFile(resolve(host.root,"../tool/estimate-started"),"utf8").then(() => true).catch(() => false)).toBe(true);
  await editSampleLabel(page,"Fresh inputs");
  await expect(cost(page)).toContainText("Waiting for the previous estimate to finish");
  await expect(cost(page)).not.toContainText("Worst case");
  await expect(page.getByRole("button",{name:"Run",exact:true})).toBeDisabled();
  await expect(estimate(page)).toBeDisabled();
  const finished = page.waitForResponse(response => new URL(response.url()).pathname.endsWith("/estimate") && response.request().method() === "POST");
  await writeFile(resolve(host.root,"../tool/release-estimate"),"release");
  await finished;
  await expect(estimate(page)).toBeEnabled();
  await expect(cost(page)).toContainText("Estimate again for the current inputs and session.");
  await expect(cost(page)).not.toContainText("Worst case");
  await expect(page.getByRole("button",{name:"Run",exact:true})).toBeDisabled();
  await estimate(page).click();
  await expect(cost(page)).toContainText("Estimate within cap.");
});

test("quote expiry clears displayed cost without estimating or running again", async ({page, host}) => {
  await page.clock.install();
  await host.open(page); await paidDraft(page,"expiry");
  let calls = 0;
  await page.route("**/api/preparations/*/estimate",async route => {
    const response = await route.fetch(); const value = await response.json(); calls++;
    value.estimate.expires = new Date(Date.now()+60000).toISOString();
    await route.fulfill({response,json:value});
  });
  try {
    await estimate(page).click();
    await expect(cost(page)).toContainText("Estimate within cap.");
    await page.clock.fastForward(120000);
    await expect(cost(page)).toContainText("Estimate again for the current inputs and session.");
    await expect(cost(page)).not.toContainText("Worst case");
  await expect(page.getByRole("button",{name:"Run",exact:true})).toBeDisabled();
    expect(calls).toBe(1);
  } finally { await page.unrouteAll({behavior:"wait"}); }
});

test("readiness refuses an unpriceable draft without showing zero cost", async ({page, host}) => {
  await host.open(page); await paidDraft(page,"readiness");
  await estimate(page).click();
  await expect(cost(page)).toContainText("Cost unavailable");
  await expect(cost(page)).not.toContainText("$0.00");
  await expect(cost(page)).toContainText("Create sample.toml");
  await expect(page.getByRole("button",{name:"Run",exact:true})).toBeDisabled();
});


test("an edit during preparation discards the late receipt without asking for a quote", async ({page, host}) => {
  await host.open(page); await paidDraft(page,"prepare held");
  let release = () => {}, received = () => {};
  const gate = new Promise<void>(resolve => { release = resolve; });
  const captured = new Promise<void>(resolve => { received = resolve; });
  let estimates = 0;
  page.on("request", request => { if (new URL(request.url()).pathname.endsWith("/estimate")) estimates++; });
  await page.route("**/api/preparations",async route => {
    const response = await route.fetch(); expect(response.status()).toBe(200);
    received(); await gate; await route.fulfill({response});
  });
  try {
    await estimate(page).click(); await captured;
    await editSampleLabel(page,"new draft");
    await expect(cost(page)).toContainText("Waiting for the previous estimate to finish");
    release(); await page.unrouteAll({behavior:"wait"});
    await expect(estimate(page)).toBeEnabled();
    await expect(cost(page)).toContainText("Estimate again");
    await expect.poll(() => readdir(resolve(host.root,"Library/execution"))).toEqual([]);
    expect(estimates).toBe(0);
  } finally { release(); await page.unrouteAll({behavior:"wait"}); }
});

test("tool restart invalidates a quote and preserves the chosen cap", async ({page, host}) => {
  await host.open(page); await paidDraft(page,"restart");
  await estimate(page).click(); await expect(cost(page)).toContainText("Estimate within cap.");
  await page.getByRole("button",{name:"Tool setup",exact:true}).click();
  const drawer = page.getByRole("dialog");
  // Session details expand above Restart; wait for that observable layout transition.
  await expect(drawer.getByRole("region",{name:"Session spending",exact:true})).toHaveAttribute("aria-busy","false");
  await drawer.getByRole("button",{name:"Restart tool session",exact:true}).click();
  await drawer.getByRole("button",{name:"Confirm restart",exact:true}).click();
  await expect(drawer.getByRole("region",{name:"Restart tool"})).toContainText("Tool restarted");
  await page.keyboard.press("Escape");
  await expect(cost(page)).toContainText("Estimate again");
  await expect(cost(page)).not.toContainText("Worst case");
  await expect(page.getByRole("button",{name:"Run",exact:true})).toBeDisabled();
  await expect(page.getByRole("textbox",{name:"Spending cap (USD)",exact:true})).toHaveValue("0.5");
  expect((await readFile(resolve(host.root,"../tool/estimates.ndjson"),"utf8")).trim().split("\n")).toHaveLength(1);
});

test("ordered image bindings and coordinates invalidate estimates", async ({page, host}) => {
  await host.open(page); await paidDraft(page,"references");
  const refs = page.getByRole("region",{name:"References (reference image)",exact:true});
  await refs.locator("input[type=file]").setInputFiles(["tests/fixtures/sample.png","tests/fixtures/panorama.png"]);
  await expect(refs).toHaveAttribute("aria-busy","false");
  await expect(refs.getByRole("img")).toHaveCount(2);
  await estimate(page).click(); await expect(cost(page)).toContainText("Estimate within cap.");
  await refs.getByRole("button",{name:"Move image 2 earlier"}).click();
  await expect(cost(page)).toContainText("Estimate again");
  await estimate(page).click(); await expect(cost(page)).toContainText("Estimate within cap.");
  await refs.getByRole("combobox").first().selectOption("equirectangular");
  await expect(cost(page)).toContainText("Estimate again");
  await estimate(page).click();
  await expect(cost(page)).toContainText("image fingerprint or frame differs from expect");
  await expect(page.getByRole("button",{name:"Run",exact:true})).toBeDisabled();
  const calls = (await readFile(resolve(host.root,"../tool/estimates.ndjson"),"utf8")).trim().split("\n").map(line => JSON.parse(line));
  expect(calls).toHaveLength(3);
  expect(calls[0].arguments.arguments.references.map((entry:{expect:{width:number}}) => entry.expect.width)).toEqual([160,2048]);
  expect(calls[1].arguments.arguments.references.map((entry:{expect:{width:number}}) => entry.expect.width)).toEqual([2048,160]);
  expect(calls[2].arguments.arguments.references[0].expect.projection).toBe("equirectangular");
});

test("configured cap rejection leaves the user's amount unchanged", async ({page, host}) => {
  await host.open(page); await paidDraft(page,"above cap");
  const cap = page.getByRole("textbox",{name:"Spending cap (USD)",exact:true});
  await cap.fill("2.0"); await estimate(page).click();
  await expect(cost(page).getByRole("alert")).toContainText("configured cap");
  await expect(cap).toHaveValue("2.0");
  await expect(cost(page)).not.toContainText("Estimate within cap.");
  await cap.fill("0.01"); await estimate(page).click();
  await expect(cost(page)).toContainText("Estimated cost exceeds this cap.");
  await expect(cap).toHaveValue("0.01");
});

test("a later ledger observation keeps the quote and warns about exhausted allowance", async ({page, host}) => {
  await host.open(page); await paidDraft(page,"first quote");
  await estimate(page).click(); await expect(cost(page)).toContainText("Estimate within cap.");
  const other = await page.context().newPage();
  try {
    await host.open(other); await paidDraft(other,"drain");
    await estimate(other).click();
    await expect(cost(other)).toContainText("remaining session budget is too low");
    await expect(cost(page)).toContainText("current session balance may no longer cover this estimate");
    await expect(cost(page)).toContainText("Estimate within cap.");
    await expect(cost(page)).toContainText("$0.12");
    expect((await readFile(resolve(host.root,"../tool/estimates.ndjson"),"utf8")).trim().split("\n")).toHaveLength(2);
  } finally { await other.close(); }
});


test("invalid JSON edits discard the quote and need an explicit estimate after correction", async ({page, host}) => {
  await host.open(page); await paidDraft(page,"JSON draft");
  await openOptionalSettings(page);
  const tags = page.getByRole("textbox",{name:"Tags · advanced",exact:true});
  await tags.fill('["original"]');
  await estimate(page).click(); await expect(cost(page)).toContainText("Estimate within cap.");
  await tags.fill('["unfinished"');
  await expect(cost(page)).not.toContainText("Worst case");
  await expect(page.getByRole("button",{name:"Run",exact:true})).toBeDisabled();
  await expect(cost(page)).toContainText("Estimate again");
  await estimate(page).click();
  await expect(tags).toBeFocused();
  await tags.fill('["corrected"]');
  await expect(cost(page)).not.toContainText("Worst case");
  await expect(page.getByRole("button",{name:"Run",exact:true})).toBeDisabled();
  expect((await readFile(resolve(host.root,"../tool/estimates.ndjson"),"utf8")).trim().split("\n")).toHaveLength(1);
  await estimate(page).click(); await expect(cost(page)).toContainText("Estimate within cap.");
});


test("a refreshed declaration invalidates its quote without losing draft inputs", async ({page, host}) => {
  await host.open(page); await paidDraft(page,"declaration");
  await estimate(page).click(); await expect(cost(page)).toContainText("Estimate within cap.");
  await writeFile(resolve(host.root,"../tool/estimate-declaration-changed"),"changed");
  await page.getByRole("button",{name:"Tool setup",exact:true}).click();
  await expect(page.getByText("Updated estimate declaration.")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(cost(page)).toContainText("Estimate again");
  await expect(cost(page)).not.toContainText("Worst case");
  await expect(page.getByRole("button",{name:"Run",exact:true})).toBeDisabled();
  await expect(page.getByLabel("Label",{exact:true})).toHaveValue("declaration");
  expect((await readFile(resolve(host.root,"../tool/estimates.ndjson"),"utf8")).trim().split("\n")).toHaveLength(1);
});
