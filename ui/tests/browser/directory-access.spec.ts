import { test, expect } from "@playwright/test";
const story = (name: string) => `http://127.0.0.1:6006/iframe.html?id=workspace-directoryaccess--${name}&viewMode=story`;

test("folder access distinguishes requested, narrowed, denied and unreported permissions", async ({ page }) => {
  await page.goto(story("narrowed"));
  const panel = page.getByRole("region", { name: "Folder access" });
  const requested = panel.getByRole("group", { name: "Requested input folders" });
  const effective = panel.getByRole("group", { name: "Effective input folders" });
  const evidence = panel.getByRole("group", { name: "Result folders" });
  await expect(requested).toContainText("/work/library/execution");
  await expect(requested).toContainText("/work/reference images");
  await expect(effective).toContainText("/work/reference images/approved");
  await expect(effective).not.toContainText("/work/library/execution");
  await expect(evidence).toContainText("/work/tool/results");
  await expect(effective).not.toContainText("/work/tool/results");
  for (const [name, expected] of [
    ["no-access", "No file paths allowed."],
    ["not-reported", "Not reported"],
    ["loading", "Checking access…"],
    ["unavailable", "Unavailable"],
  ]) {
    await page.goto(story(name!));
    await expect(effective).toContainText(expected!);
    await expect(effective.locator("code")).toHaveCount(0);
    await expect(effective).not.toContainText("/work/library/execution");
    if (name !== "no-access") await expect(effective).not.toContainText("No file paths allowed.");
  }
  await page.goto(story("no-access"));
  await expect(requested).toContainText("No folders requested.");
  await expect(evidence).toContainText("No result folders granted.");
});

test("long literal folder names fit narrow screens without becoming markup", async ({ page }, testInfo) => {
  const failures: string[] = [];
  page.on("pageerror", error => failures.push(error.message));
  page.on("console", message => { if (["warning", "error"].includes(message.type())) failures.push(message.text()); });
  await page.goto(story("long-paths"));
  const panel = page.getByRole("region", { name: "Folder access" });
  await expect(panel).toContainText("<literal> $(name)");
  await expect(panel.locator("literal")).toHaveCount(0);
  for (const width of [320, 768, 1024, 1440]) {
    await page.setViewportSize({width, height: 1100});
    await expect(panel.getByRole("heading", {name:"Folder access"})).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await panel.screenshot({path:testInfo.outputPath(`directory-access-${width}.png`)});
  }
  expect(failures).toEqual([]);
});
