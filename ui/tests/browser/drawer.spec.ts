import { test, expect } from "@playwright/test";
test("shared drawer contains focus, scrolls and returns to its trigger", async ({
  page,
}) => {
  await page.goto(
    "http://127.0.0.1:6006/iframe.html?id=components-drawer--inspection&viewMode=story",
  );
  const trigger = page.getByRole("button", { name: "Inspect run" });
  await trigger.click();
  const dialog = page.getByRole("dialog", { name: "Run details" });
  await expect(dialog).toBeVisible();
  await expect(
    dialog.getByRole("heading", { name: "Run details" }),
  ).toBeFocused();
  await trigger.evaluate((element) => element.focus());
  await expect(trigger).not.toBeFocused();
  await page.keyboard.press("Escape");
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
