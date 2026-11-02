import { test, expect } from "./tool-host";
import { prepareSampleInput } from "./sample-input";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import type { Page } from "@playwright/test";
test.use({hostScenario:"estimates"});
const outcome = (page:Page) => page.getByRole("region",{name:"Run outcome",exact:true});
async function prepare(page:Page,label:string) {
  await page.getByRole("button",{name:/^Action /}).click();
  await page.getByRole("button",{name:/^Price a scripted call/}).click();
  await prepareSampleInput(page,label);
  await page.getByRole("textbox",{name:"Spending cap (USD)",exact:true}).fill("0.5");
  await page.getByRole("button",{name:"Estimate cost",exact:true}).click();
  await expect(page.getByRole("region",{name:"Cost",exact:true})).toContainText("Estimate within cap.");
}
for (const [label,title,spend] of [
  ["paid-stale","Estimate no longer matches.","$0.00"],
  ["paid-cap","Spending cap reached.","$0.00"],
  ["paid-credential","Provider rejected authentication.","$0.00"],
  ["paid-unresolved","Run failed.","At most $0.12"],
  ["paid-failed","Run failed.","$0.08"],
] as const) test(`${label} shows saved outcome and spend without retrying`,async ({page,host},testInfo) => {
  await host.open(page); await prepare(page,label);
  await page.getByRole("button",{name:"Run",exact:true}).click();
  await expect(outcome(page)).toContainText(title);
  await expect(outcome(page).locator("dd")).toHaveText(spend);
  await expect(page.getByRole("region",{name:"Result",exact:true})).toContainText("Contract evidence: complete");
  await expect(page.getByRole("button",{name:"Run",exact:true})).toBeDisabled();
  if (label === "paid-unresolved") {
    await expect(outcome(page)).toContainText("final charge is unknown");
    await outcome(page).scrollIntoViewIfNeeded();
    await page.screenshot({path:testInfo.outputPath("unresolved-run.png")});
  }
  if (label === "paid-credential") {
    await outcome(page).getByRole("button",{name:"Review tool setup"}).click();
    await expect(page.getByRole("dialog")).toContainText("SCRIPTED_PAID_KEY");
    await expect(page.getByRole("dialog")).toContainText("Invalid");
    await page.keyboard.press("Escape");
  }
  expect((await readFile(resolve(host.root,"../tool/calls.ndjson"),"utf8")).trim().split("\n")).toHaveLength(1);
});

test("missing setup is a no-run refusal with zero spend and requirements",async ({page,host}) => {
  await host.open(page); await prepare(page,"paid-not-ready");
  await page.getByRole("button",{name:"Run",exact:true}).click();
  await expect(outcome(page)).toContainText("No run created.");
  await expect(outcome(page)).toContainText("SCRIPTED_PAID_KEY");
  await expect(outcome(page).locator("dd")).toHaveText("$0.00");
  await expect(outcome(page)).not.toContainText("Provider rejected authentication");
  expect(await readFile(resolve(host.root,"../tool/calls.ndjson"),"utf8").catch(() => "")).toBe("");
});

test("two reviewed drafts explain a later budget refusal without resetting the session",async ({page,host}) => {
  await host.open(page); await prepare(page,"paid-full");
  const second = await page.context().newPage();
  try {
    await host.open(second); await prepare(second,"second paid draft");
    await page.getByRole("button",{name:"Run",exact:true}).click();
    await expect(page.getByRole("region",{name:"Result",exact:true})).toContainText("Contract evidence: complete");
    await expect(second.getByRole("region",{name:"Cost",exact:true})).toContainText("balance may no longer cover this estimate");
    await second.getByRole("button",{name:"Run",exact:true}).click();
    await expect(outcome(second)).toContainText("Session budget reached.");
    await expect(outcome(second).locator("dd")).toHaveText("$0.00");
    await outcome(second).getByRole("button",{name:"Review tool setup"}).click();
    const spending = second.getByRole("region",{name:"Session spending"});
    await expect(spending).toContainText("$0.12");
    await expect(spending).toContainText("$0.08");
    expect((await readFile(resolve(host.root,"../tool/calls.ndjson"),"utf8")).trim().split("\n")).toHaveLength(2);
  } finally { await second.close(); }
});

test("saved outcome setup stays attached to its action while the current draft changes",async ({page,host}) => {
  await host.open(page); await prepare(page,"paid-credential");
  let first = true;
  await page.route("**/api/preparations/*/submit",async route => {
    const response = await route.fetch();
    if (first) { first = false; await route.abort("failed"); }
    else await route.fulfill({response});
  });
  try {
    await page.getByRole("button",{name:"Run",exact:true}).click();
    await expect(page.getByRole("button",{name:"Retry submission",exact:true})).toBeVisible();
    await page.getByRole("button",{name:/^Action /}).click();
    await page.getByRole("button",{name:/^Add a test band/}).click();
    await page.getByRole("button",{name:"Retry submission",exact:true}).click();
    await expect(outcome(page)).toContainText("Provider rejected authentication.");
    await outcome(page).getByRole("button",{name:"Review tool setup"}).click();
    await expect(page.getByRole("dialog")).toContainText("Price a scripted call");
    await expect(page.getByRole("dialog")).toContainText("Invalid");
    await page.keyboard.press("Escape");
    await expect(page.getByRole("button",{name:/^Action /})).toContainText("Add a test band");
  } finally { await page.unrouteAll({behavior:"wait"}); }
});

test("run outcome stories explain uncertainty and fit a narrow viewport",async ({page}) => {
  await page.setViewportSize({width:375,height:812});
  for (const [story,text] of [["cancelled","At most $0.20"],["interrupted","Unavailable"],["no-run","No run created."]]) {
    await page.goto(`http://127.0.0.1:6006/iframe.html?id=workspace-runoutcome--${story}&viewMode=story`);
    await expect(outcome(page)).toContainText(text!);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  }
  await outcome(page).getByRole("button",{name:"Review tool setup"}).click();
  await expect(page.getByText("Setup requested",{exact:true})).toBeVisible();
});
