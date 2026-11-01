import { test, expect } from "@playwright/test";
test("comparison divider supports keyboard boundaries and side-by-side mode", async ({
  page,
}) => {
  await page.goto(
    "http://127.0.0.1:6006/iframe.html?id=components-imagecompare--wipe&viewMode=story",
  );
  const slider = page.getByRole("slider", { name: "Before and after divider" });
  await expect(slider).toBeVisible();
  await slider.focus();
  await page.keyboard.press("Home");
  await expect(slider).toHaveValue("0");
  await page.keyboard.press("End");
  await expect(slider).toHaveValue("100");
  await page.keyboard.press("ArrowLeft");
  await expect(slider).toHaveValue("99");
  await page.getByRole("button", { name: "Side by side" }).click();
  await expect(slider).not.toBeVisible();
  await expect(page.getByRole("img")).toHaveCount(2);
  await page.setViewportSize({ width: 320, height: 640 });
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth),
  ).toBeLessThanOrEqual(320);
});
test("incompatible and missing image frames have explicit states", async ({
  page,
}) => {
  await page.goto(
    "http://127.0.0.1:6006/iframe.html?id=components-imagecompare--incompatible&viewMode=story",
  );
  await expect(
    page.getByRole("button", { name: "Wipe", exact: true }),
  ).toBeDisabled();
  await expect(page.getByText(/different dimensions/)).toBeVisible();
  await page.goto(
    "http://127.0.0.1:6006/iframe.html?id=components-imagecompare--unavailable&viewMode=story",
  );
  await expect(page.getByText(/saved bytes could not be read/)).toHaveCount(2);
});
