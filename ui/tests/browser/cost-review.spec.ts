import { test, expect } from "./fixtures";
const story = (name: string) => `http://127.0.0.1:6006/iframe.html?id=workspace-costreview--${name}&viewMode=story`;

test("cap edits require positive values and estimating is an explicit keyboard action", async ({ page }) => {
  await page.goto(story("interactive"));
  const cap = page.getByRole("textbox", { name: "Spending cap (USD)", exact: true });
  const estimate = page.getByRole("button", { name: "Estimate cost", exact: true });
  await expect(estimate).toBeDisabled();
  for (const value of ["0", "-1", "Infinity", "1e999", "not a number"]) {
    await cap.fill(value);
    await expect(cap).toHaveValue(value);
    await expect(estimate).toBeDisabled();
    await expect(page.getByText("Enter a finite amount greater than zero.")).toBeVisible();
  }
  await cap.fill("1.");
  await expect(cap).toHaveValue("1.");
  await expect(estimate).toBeEnabled();
  await expect(page.getByText("Estimates requested: 0")).toBeVisible();
  await estimate.focus(); await page.keyboard.press("Enter");
  await expect(page.getByText("Estimates requested: 1")).toBeVisible();
  await cap.fill(".5");
  await expect(page.getByText("Estimates requested: 1")).toBeVisible();
  await expect(page.getByRole("button", { name: "Run", exact: true })).toHaveCount(0);
});

test("cost states preserve unknowns, refusals and current allowance without claiming free work", async ({ page }) => {
  await page.goto(story("ready"));
  const cost = page.getByRole("region", { name: "Cost", exact: true });
  await expect(cost).toContainText("$0.12");
  await expect(cost).toContainText("$1.80");
  await expect(cost).toContainText("scripted-model");
  await expect(cost).toContainText("$0.04 whose final charge is unknown");
  await page.getByText("Estimate details", {exact:true}).click();
  await expect(cost).toContainText(`sha256:${"a".repeat(64)}`);
  await page.goto(story("needs-setup"));
  await expect(cost).toContainText("Cost unavailable");
  await expect(cost).not.toContainText("$0.00");
  await expect(cost).toContainText("MODEL_DIR");
  await expect(cost).toContainText("Set MODEL_DIR before starting the tool.");
  await page.getByRole("button", {name:"Tool setup",exact:true}).click();
  await expect(page.getByText("Setup requested")).toBeVisible();
  await page.goto(story("over-cap"));
  await expect(cost).toContainText("Estimated cost exceeds this cap.");
  await expect(page.getByRole("textbox", {name:"Spending cap (USD)"})).toHaveValue("0.01");
  await page.goto(story("budget-refused"));
  await expect(cost).toContainText("The tool’s remaining session budget is too low.");
  await page.goto(story("uncapped"));
  await expect(cost).toContainText("No session limit");
  await page.goto(story("tiny-cost"));
  await expect(cost).toContainText("< $0.000001");
  await page.goto(story("pending"));
  await expect(page.getByRole("button", {name:"Estimate cost"})).toBeDisabled();
  await expect(cost).toContainText("Estimating these inputs…");
  await page.goto(story("stale"));
  await expect(cost).toContainText("Estimate again for the current inputs and session.");
  await expect(cost).not.toContainText("$0.12");
  await page.goto(story("failed"));
  await expect(page.getByRole("alert")).toContainText("The cap exceeds the configured limit.");
  await expect(page.getByRole("textbox", {name:"Spending cap (USD)"})).toHaveValue("2");
});

test("cost panel fits a narrow input column and wide layouts", async ({ page }, testInfo) => {
  await page.goto(story("ready"));
  await expect(page.getByRole("region", {name:"Cost"})).toContainText("$0.12");
  await page.getByText("Estimate details", {exact:true}).click();
  for (const width of [320, 768, 1440]) {
    await page.setViewportSize({width, height:900});
    await expect(page.getByRole("textbox", {name:"Spending cap (USD)"})).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({path:testInfo.outputPath(`cost-${width}.png`),fullPage:true});
  }
});

test("ledger changes remain observations while new sessions hide the old estimate", async ({ page }) => {
  await page.goto(story("ledger-changed"));
  const cost = page.getByRole("region", {name:"Cost",exact:true});
  await expect(cost).toContainText("Estimate within cap.");
  await expect(cost).toContainText("$0.12");
  await expect(cost).toContainText("$0.00");
  await expect(cost).toContainText("The current session balance may no longer cover this estimate.");
  await page.goto(story("session-changed"));
  await expect(cost).toContainText("Estimate again for the current inputs and session.");
  await expect(cost).not.toContainText("$0.12");
  await page.goto(story("unavailable"));
  await expect(cost).toContainText("This action is unavailable in the current tool environment.");
  await expect(cost).toContainText("This build does not include the backend.");
});
