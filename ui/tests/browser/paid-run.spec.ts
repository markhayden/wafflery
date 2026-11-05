import { test, expect } from "./tool-host";
import { editSampleLabel, openCost, prepareSampleInput } from "./sample-input";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import type { Page } from "@playwright/test";
test.use({hostScenario:"estimates"});
const run = (page: Page) => page.getByRole("button",{name:"Run",exact:true});
const cost = (page: Page) => page.getByRole("region",{name:"Cost options",exact:true});
const summary = (page: Page) => page.getByText(/^May cost money · /);
async function paidDraft(page: Page, label = "reviewed paid draft") {
  await page.getByRole("button",{name:/^Action /}).click();
  await page.getByRole("button",{name:/^Price a scripted call/}).click();
  await prepareSampleInput(page,label);
}
/** Count how Run reached the host: plain submissions, bound (checked) submissions, preparations. */
function track(page: Page) {
  const seen = {plain:0,bound:0,prepared:0};
  page.on("request",request => {
    if (request.method() !== "POST") return;
    const path = new URL(request.url()).pathname;
    if (path === "/api/attempts") seen.plain++;
    else if (/^\/api\/preparations\/[^/]+\/submit$/.test(path)) seen.bound++;
    else if (path === "/api/preparations") seen.prepared++;
  });
  return seen;
}
async function quote(page: Page) {
  await paidDraft(page);
  await openCost(page);
  await page.getByRole("textbox",{name:"Spending limit (USD) Optional",exact:true}).fill("0.5");
  // Checking cost is optional: Run is available before the check.
  await expect(run(page)).toBeEnabled();
  await page.getByRole("button",{name:"Check cost",exact:true}).click();
  await expect(cost(page)).toContainText("Within the limit. Running uses this price.");
  await expect(run(page)).toBeEnabled();
}
async function calls(root: string) {
  return (await readFile(resolve(root,"../tool/calls.ndjson"),"utf8")).trim().split("\n").map(line => JSON.parse(line));
}
test("an unchecked paid Run is a plain submission and the tool's own limits apply",async ({page,host}) => {
  const seen = track(page);
  await host.open(page); await paidDraft(page,"unchecked paid draft");
  await expect(summary(page)).toHaveText("May cost money · the tool's spending limits apply");
  await expect(page.getByRole("region",{name:"Cost options",exact:true})).toHaveCount(0);
  await expect(run(page)).toBeEnabled();
  await run(page).click();
  await expect(page.getByRole("region",{name:"Result",exact:true})).toContainText("Contract evidence: complete");
  expect(seen).toEqual({plain:1,bound:0,prepared:0});
  const sent = await calls(host.root);
  expect(sent).toHaveLength(1);
  expect(sent[0].arguments.label).toBe("unchecked paid draft");
  // No limit was typed, so none is sent; nothing binds the call to an estimate.
  expect(sent[0].arguments).not.toHaveProperty("max_spend_usd");
  expect(sent[0]._meta?.["table-manners/estimate"]).toBeUndefined();
});

test("a typed spending limit is sent with an unchecked Run and a non-positive one is refused",async ({page,host}) => {
  const seen = track(page);
  await host.open(page); await paidDraft(page,"limited paid draft");
  const panel = await openCost(page);
  const limit = page.getByRole("textbox",{name:"Spending limit (USD) Optional",exact:true});
  await expect(limit).not.toHaveAttribute("required");
  await limit.fill("0");
  await expect(panel.getByRole("button",{name:"Check cost",exact:true})).toBeDisabled();
  // The field is invalid, so the form's own validation stops Run before anything is sent.
  await run(page).click();
  await expect(limit).toHaveAttribute("aria-invalid","true");
  await expect(panel).toContainText("Enter an amount above zero, or leave it empty.");
  expect(await limit.evaluate((node: HTMLInputElement) => node.validity.valid)).toBe(false);
  expect(seen).toEqual({plain:0,bound:0,prepared:0});
  await limit.fill("0.5");
  await expect(summary(page)).toHaveText("May cost money · your limit $0.50");
  await run(page).click();
  await expect(page.getByRole("region",{name:"Result",exact:true})).toContainText("Contract evidence: complete");
  expect(seen).toEqual({plain:1,bound:0,prepared:0});
  const sent = await calls(host.root);
  expect(sent).toHaveLength(1);
  expect(sent[0].arguments.max_spend_usd).toBe(0.5);
  expect(sent[0]._meta?.["table-manners/estimate"]).toBeUndefined();
});

test("a check made stale by an edit never binds: Run then submits the edited draft unbound",async ({page,host}) => {
  const seen = track(page);
  await host.open(page); await quote(page);
  await editSampleLabel(page,"edited after the check");
  await expect(cost(page)).toContainText("The inputs changed since the last check. Check again, or just run.");
  await expect(summary(page)).toHaveText("May cost money · your limit $0.50");
  await run(page).click();
  await expect(page.getByRole("region",{name:"Result",exact:true})).toContainText("Contract evidence: complete");
  expect(seen).toEqual({plain:1,bound:0,prepared:1});
  const sent = await calls(host.root);
  expect(sent).toHaveLength(1);
  expect(sent[0].arguments.label).toBe("edited after the check");
  expect(sent[0]._meta?.["table-manners/estimate"]).toBeUndefined();
});

test("a checked paid Run submits the priced draft once, bound; another Run is unbound unless checked again",async ({page,host},testInfo) => {
  const requests: unknown[] = [];
  const seen = track(page);
  page.on("request",request => {
    if (new URL(request.url()).pathname.endsWith("/submit")) requests.push(request.postDataJSON());
  });
  await host.open(page); await quote(page);
  await run(page).click();
  const result = page.getByRole("region",{name:"Result",exact:true});
  await expect(result).toContainText("Completed");
  await expect(result).toContainText("Contract evidence: complete");
  // The check was spent on that run: the panel says so and Run stays available.
  await expect(cost(page)).toContainText("Check again");
  await expect(summary(page)).toHaveText("May cost money · your limit $0.50");
  await expect(run(page)).toBeEnabled();
  expect(requests).toHaveLength(1);
  expect(seen).toEqual({plain:0,bound:1,prepared:1});
  const sent = await calls(host.root);
  expect(sent).toHaveLength(1);
  expect(sent[0].arguments.label).toBe("reviewed paid draft");
  expect(sent[0].arguments.max_spend_usd).toBe(0.5);
  expect(sent[0]._meta["table-manners/estimate"]).toMatch(/^sha256:/);
  expect(requests[0]).not.toHaveProperty("binding");
  await page.getByRole("button",{name:"Inspect run",exact:true}).click();
  await page.getByRole("button",{name:"Advanced",exact:true}).click();
  await page.getByRole("button",{name:"Recorded call",exact:true}).click();
  await expect(page.getByRole("dialog")).toContainText(sent[0]._meta["table-manners/estimate"]);
  await page.keyboard.press("Escape");
  // Without a new check, another Run is a plain submission of the same inputs.
  await run(page).click();
  await expect.poll(async () => (await calls(host.root)).length).toBe(2);
  await expect(result).toContainText("Contract evidence: complete");
  expect(seen).toEqual({plain:1,bound:1,prepared:1});
  const second = (await calls(host.root))[1];
  expect(second.arguments.label).toBe("reviewed paid draft");
  expect(second._meta?.["table-manners/estimate"]).toBeUndefined();
  // Checking again prices a fresh preparation without running anything. Two runs have spent
  // $0.16 of the scenario's $0.20 session budget, so this check is refused, with its reason.
  await page.getByRole("button",{name:"Check cost",exact:true}).click();
  await expect(cost(page)).toContainText("The tool’s remaining session budget is too low.");
  await expect(cost(page)).toContainText("Left to spend this session: $0.04 of $0.20");
  await expect(summary(page)).toHaveText("May cost money · the check was refused (see Cost options)");
  await page.screenshot({path:testInfo.outputPath("reviewed-paid-run.png")});
  expect(await calls(host.root)).toHaveLength(2);
  expect(seen).toEqual({plain:1,bound:1,prepared:2});
});

for (const mode of ["lost", "wrong", "missing declaration"] as const) test(`${mode} paid acknowledgement preserves the original id across edits and explicit retry`,async ({page,host}) => {
  await host.open(page); await quote(page);
  const submitted: {id:string;params:{label:string}}[] = [];
  let first = true;
  await page.route("**/api/preparations/*/submit",async route => {
    submitted.push(route.request().postDataJSON());
    const response = await route.fetch();
    expect(response.status()).toBe(202);
    if (first) {
      first = false;
      if (mode === "wrong") {
        const value = await response.json(); value.request.params.label = "wrong response";
        await route.fulfill({response,json:value});
      } else await route.abort("failed");
    }
    else await route.fulfill({response});
  });
  try {
    await run(page).click();
    const retry = page.getByRole("button",{name:"Retry submission",exact:true});
    await expect(retry).toBeVisible();
    await editSampleLabel(page,"edited after sending");
    await expect(page.getByRole("button",{name:"Check cost",exact:true})).toBeDisabled();
    expect(submitted).toHaveLength(1);
    if (mode === "missing declaration") {
      await page.route("**/api/state",async route => {
        const response = await route.fetch(); const value = await response.json();
        value.tools[0].declaration = null;
        await route.fulfill({response,json:value});
      });
      await page.getByRole("button",{name:"Tool setup",exact:true}).click();
      await expect(page.getByRole("dialog")).toContainText("Setup is unavailable because this tool or action is no longer available.");
      await page.keyboard.press("Escape");
      await expect(page.getByText("No supported action is available.")).toBeVisible();
    }
    await retry.click();
    await expect(page.getByRole("region",{name:"Result",exact:true})).toContainText("Contract evidence: complete");
    await expect(retry).toHaveCount(0);
    expect(submitted).toHaveLength(2);
    expect(submitted[1]).toEqual(submitted[0]);
    expect((await calls(host.root))).toHaveLength(1);
    expect(submitted[0]!.params.label).toBe("reviewed paid draft");
    if (mode !== "missing declaration") await expect(page.getByLabel("Label",{exact:true})).toHaveValue("edited after sending");
    if (mode !== "missing declaration") await expect(run(page)).toBeEnabled();
  } finally { await page.unrouteAll({behavior:"wait"}); }
});

test("edits while paid submission is in flight cannot delete or replace its prepared inputs",async ({page,host}) => {
  await host.open(page); await quote(page);
  let release = () => {}, entered = () => {};
  const gate = new Promise<void>(resolve => { release = resolve; });
  const started = new Promise<void>(resolve => { entered = resolve; });
  let posts = 0, deletes = 0;
  page.on("request",request => { if (request.method() === "DELETE" && request.url().includes("/api/preparations/")) deletes++; });
  await page.route("**/api/preparations/*/submit",async route => {
    posts++; entered(); await gate;
    const response = await route.fetch(); await route.fulfill({response});
  });
  try {
    await run(page).click(); await started;
    await editSampleLabel(page,"next draft");
    await expect(run(page)).toBeDisabled();
    await expect(page.getByRole("button",{name:"Check cost",exact:true})).toBeDisabled();
    expect(deletes).toBe(0);
    release(); await page.unrouteAll({behavior:"wait"});
    await expect(page.getByRole("region",{name:"Result",exact:true})).toContainText("Contract evidence: complete");
    expect(posts).toBe(1); expect(deletes).toBe(0);
    expect((await calls(host.root))[0].arguments.label).toBe("reviewed paid draft");
    await expect(page.getByLabel("Label",{exact:true})).toHaveValue("next draft");
    // Once it settles, the edited draft can run (unbound) like any other.
    await expect(run(page)).toBeEnabled();
  } finally { release(); await page.unrouteAll({behavior:"wait"}); }
});

test("a definitive host refusal discards the bound inputs; a new check prepares afresh",async ({page,host}) => {
  const preparations: string[] = [];
  page.on("request",request => {
    if (new URL(request.url()).pathname === "/api/preparations" && request.method() === "POST") preparations.push(request.postDataJSON().id);
  });
  await host.open(page); await quote(page);
  await page.route("**/api/preparations/*/submit",route => route.fulfill({status:409,
    json:{error:{message:"The prepared inputs changed. Estimate again."}}}));
  try {
    await run(page).click();
    await expect(page.getByRole("alert")).toContainText("prepared inputs changed");
    await expect(page.getByRole("button",{name:"Retry submission",exact:true})).toHaveCount(0);
    await expect(cost(page)).toContainText("Check again");
    await expect(cost(page)).not.toContainText("Worst case");
    await expect(run(page)).toBeEnabled();
  } finally { await page.unrouteAll({behavior:"wait"}); }
  await page.getByRole("button",{name:"Check cost",exact:true}).click();
  await expect(cost(page)).toContainText("Within the limit. Running uses this price.");
  await expect(run(page)).toBeEnabled();
  expect(preparations).toHaveLength(2); expect(preparations[0]).not.toBe(preparations[1]);
  await run(page).click();
  await expect(page.getByRole("region",{name:"Result",exact:true})).toContainText("Contract evidence: complete");
  const sent = await calls(host.root);
  expect(sent).toHaveLength(1);
  expect(sent[0]._meta["table-manners/estimate"]).toMatch(/^sha256:/);
});
