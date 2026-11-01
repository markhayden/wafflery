import { test, expect } from "@playwright/test";
test("focused picker searches a long list, selects and restores focus", async ({
  page,
}) => {
  await page.goto(
    "http://127.0.0.1:6006/iframe.html?id=components-picker--long-list&viewMode=story",
  );
  const trigger = page.locator("button[aria-haspopup=dialog]");
  await trigger.click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await expect(
    page.getByRole("textbox", { name: "Search action" }),
  ).toBeFocused();
  await page.getByRole("textbox").fill("Action 47");
  await page.getByRole("button", { name: /Action 47/ }).click();
  await expect(page.getByRole("dialog")).not.toBeVisible();
  await expect(trigger).toBeFocused();
  await expect(trigger).toContainText("Action 47");
  await trigger.click();
  await page.keyboard.press("Escape");
  await expect(trigger).toBeFocused();
  await page.setViewportSize({ width: 320, height: 640 });
  await trigger.click();
  expect(
    await page.evaluate(() => document.documentElement.scrollWidth),
  ).toBeLessThanOrEqual(320);
});
test("declared form keeps arrays as JSON and invalid edits block submission", async ({
  page,
}) => {
  await page.goto(
    "http://127.0.0.1:6006/iframe.html?id=inputs-actionform--declared&viewMode=story",
  );
  await page.getByLabel("prompt *").fill("A seam");
  await page.getByLabel("enabled").selectOption("true");
  await page.getByLabel("tags").fill('["one","two"]');
  await page.getByRole("button", { name: "Validate form" }).click();
  await expect(page.locator("output")).toHaveText(
    '{"prompt":"A seam","enabled":true,"tags":["one","two"]}',
  );
  await page.getByLabel("tags").fill('["broken"');
  await page.getByLabel("prompt *").fill("Changed");
  await page.getByRole("button", { name: "Validate form" }).click();
  await expect(page.locator("output")).not.toContainText("Changed");
  await expect(page.getByRole("alert")).toBeVisible();
});
