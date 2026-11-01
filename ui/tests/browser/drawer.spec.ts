import { test, expect } from "@playwright/test";
test("shared drawer contains focus, scrolls and returns to its trigger", async ({
  page,
}, testInfo) => {
  await page.goto(
    "http://127.0.0.1:6006/iframe.html?id=components-drawer--inspection&viewMode=story",
  );
  const trigger = page.getByRole("button", { name: "Inspect run" });
  await trigger.click();
  const dialog = page.getByRole("dialog", { name: "Run details" });
  await expect(dialog).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath("drawer.png") });
  await expect(
    dialog.getByRole("heading", { name: "Run details" }),
  ).toBeFocused();
  await trigger.evaluate((element) => element.focus());
  await expect(trigger).not.toBeFocused();
  await page.keyboard.press("Escape");
  await expect(dialog).not.toBeVisible();
  await expect(trigger).toBeFocused();
  await trigger.click();
  await dialog.getByRole("heading", { name: "Run details" }).click();
  await expect(dialog).toBeVisible();
  const bounds = (await dialog.boundingBox())!;
  // A selection/drag that begins inside must not dismiss when released outside.
  await page.mouse.move(bounds.x + 30, 100);
  await page.mouse.down();
  await page.mouse.move(20, 100);
  await page.mouse.up();
  await expect(dialog).toBeVisible();
  await page.mouse.click(20, 100);
  await expect(dialog).not.toBeVisible();
  await expect(trigger).toBeFocused();
  await page.setViewportSize({ width: 320, height: 640 });
  await trigger.click();
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth),
  ).toBeLessThanOrEqual(320);
  await dialog
    .getByRole("button", { name: "Last action" })
    .scrollIntoViewIfNeeded();
  await expect(dialog.getByRole("button", { name: "Close" })).toBeVisible();
  await dialog.getByRole("button", { name: "Close" }).click();
  await expect(trigger).toBeFocused();
});
