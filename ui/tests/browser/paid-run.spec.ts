import { test, expect } from "./tool-host";
import { editSampleLabel, prepareSampleInput } from "./sample-input";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import type { Page } from "@playwright/test";
test.use({hostScenario:"estimates"});
const run = (page: Page) => page.getByRole("button",{name:"Run",exact:true});
const cost = (page: Page) => page.getByRole("region",{name:"Cost",exact:true});
async function quote(page: Page) {
  await page.getByRole("button",{name:/^Action /}).click();
  await page.getByRole("button",{name:/^Price a scripted call/}).click();
  await prepareSampleInput(page,"reviewed paid draft");
  await page.getByRole("textbox",{name:"Spending cap (USD)",exact:true}).fill("0.5");
  await expect(run(page)).toBeDisabled();
  await page.getByRole("button",{name:"Estimate cost",exact:true}).click();
  await expect(cost(page)).toContainText("Estimate within cap.");
  await expect(run(page)).toBeEnabled();
}
async function calls(root: string) {
  return (await readFile(resolve(root,"../tool/calls.ndjson"),"utf8")).trim().split("\n").map(line => JSON.parse(line));
}
test("explicit paid Run submits the priced draft once and another run needs a fresh estimate",async ({page,host},testInfo) => {
  const requests: unknown[] = [];
  page.on("request",request => {
    if (new URL(request.url()).pathname.endsWith("/submit")) requests.push(request.postDataJSON());
  });
  await host.open(page); await quote(page);
  await run(page).click();
  const result = page.getByRole("region",{name:"Result",exact:true});
  await expect(result).toContainText("completed");
  await expect(result).toContainText("Contract evidence: complete");
  await expect(run(page)).toBeDisabled();
  await expect(cost(page)).toContainText("Estimate again");
  expect(requests).toHaveLength(1);
  const sent = await calls(host.root);
  expect(sent).toHaveLength(1);
  expect(sent[0].arguments.label).toBe("reviewed paid draft");
  expect(sent[0].arguments.max_spend_usd).toBe(0.5);
  expect(sent[0]._meta["table-manners/estimate"]).toMatch(/^sha256:/);
  expect(requests[0]).not.toHaveProperty("binding");
  await page.getByRole("button",{name:"Inspect run",exact:true}).click();
  await page.getByRole("button",{name:"Recorded call",exact:true}).click();
  await expect(page.getByRole("dialog")).toContainText(sent[0]._meta["table-manners/estimate"]);
  await page.keyboard.press("Escape");
  await page.getByRole("button",{name:"Estimate cost",exact:true}).click();
  await expect(cost(page)).toContainText("Estimate within cap.");
  await expect(run(page)).toBeEnabled();
  await page.screenshot({path:testInfo.outputPath("reviewed-paid-run.png")});
  expect(await calls(host.root)).toHaveLength(1);
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
    await expect(page.getByRole("button",{name:"Estimate cost",exact:true})).toBeDisabled();
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
    if (mode !== "missing declaration") await expect(page.getByLabel("Label *",{exact:true})).toHaveValue("edited after sending");
    await expect(run(page)).toBeDisabled();
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
    await expect(page.getByRole("button",{name:"Estimate cost",exact:true})).toBeDisabled();
    expect(deletes).toBe(0);
    release(); await page.unrouteAll({behavior:"wait"});
    await expect(page.getByRole("region",{name:"Result",exact:true})).toContainText("Contract evidence: complete");
    expect(posts).toBe(1); expect(deletes).toBe(0);
    expect((await calls(host.root))[0].arguments.label).toBe("reviewed paid draft");
    await expect(page.getByLabel("Label *",{exact:true})).toHaveValue("next draft");
    await expect(run(page)).toBeDisabled();
  } finally { release(); await page.unrouteAll({behavior:"wait"}); }
});

test("a definitive host refusal requires a fresh preparation and explicit estimate",async ({page,host}) => {
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
    await expect(run(page)).toBeDisabled();
    await expect(page.getByRole("button",{name:"Retry submission",exact:true})).toHaveCount(0);
    await expect(cost(page)).toContainText("Estimate again");
  } finally { await page.unrouteAll({behavior:"wait"}); }
  await page.getByRole("button",{name:"Estimate cost",exact:true}).click();
  await expect(cost(page)).toContainText("Estimate within cap.");
  await expect(run(page)).toBeEnabled();
  expect(preparations).toHaveLength(2); expect(preparations[0]).not.toBe(preparations[1]);
  await run(page).click();
  await expect(page.getByRole("region",{name:"Result",exact:true})).toContainText("Contract evidence: complete");
  expect(await calls(host.root)).toHaveLength(1);
});
