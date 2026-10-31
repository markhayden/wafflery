import { test, expect } from "./fixtures";

for (const width of [320, 768, 1440]) {
  test(`workspace stays readable at ${width}px`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "Your workshop" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Connect a tool" })).toBeDisabled();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
    await page.screenshot({ path: testInfo.outputPath("workspace.png"), fullPage: true });
  });
}

test("shared button supports keyboard focus", async ({ page, browserName }) => {
  await page.goto("http://127.0.0.1:6006/iframe.html?id=controls-button--default&viewMode=story");
  const button = page.getByRole("button", { name: "Run", exact: true });
  await expect(button).toBeVisible();
  // macOS WebKit follows Safari’s Option-Tab navigation for all controls.
  // https://support.apple.com/guide/safari/keyboard-shortcuts-and-gestures-cpsh003/mac
  await page.keyboard.press(browserName === "webkit" && process.platform === "darwin" ? "Alt+Tab" : "Tab");
  await expect(button).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(button).toBeFocused();
});

test("busy button cannot dispatch another action", async ({ page }) => {
  await page.goto("http://127.0.0.1:6006/iframe.html?id=controls-button--busy&viewMode=story");
  const button = page.getByRole("button", { name: "Running…" });
  await expect(button).toBeDisabled();
  await expect(button).toHaveAttribute("aria-busy", "true");
});

test("long button label wraps at narrow widths", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 600 });
  await page.goto("http://127.0.0.1:6006/iframe.html?id=controls-button--long-label&viewMode=story");
  await expect(page.getByRole("button", { name: /Download this run/ })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(320);
});
