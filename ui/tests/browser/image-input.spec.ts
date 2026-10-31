import { test, expect } from "./fixtures";

test("multiple images can be reordered with the keyboard", async ({ page }) => {
  await page.goto("http://127.0.0.1:6006/iframe.html?id=inputs-imageinput--multiple&viewMode=story");
  const earlier = page.getByRole("button", { name: "Move image 2 earlier" });
  await earlier.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("listitem").first()).toContainText("320 × 100");
  await expect(page.getByRole("button", { name: "Move image 1 earlier" })).toBeDisabled();
});

test("long image lists and labels fit narrow layouts", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 320, height: 900 });
  await page.goto("http://127.0.0.1:6006/iframe.html?id=inputs-imageinput--long-list&viewMode=story");
  await expect(page.getByRole("img")).toHaveCount(8);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(320);
  await page.screenshot({ path: testInfo.outputPath("image-input-long.png"), fullPage: true });
});

test("upload failures are visible and preserve an editable input", async ({ page }) => {
  await page.goto("http://127.0.0.1:6006/iframe.html?id=inputs-imageinput--upload-error&viewMode=story");
  await page.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
  await expect(page.getByRole("alert")).toHaveText("Not enough disk space to save this image.");
  await expect(page.locator("input[type=file]")).toBeEnabled();
});
