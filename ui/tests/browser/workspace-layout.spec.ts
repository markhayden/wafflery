import { prepareSampleInput } from "./sample-input";
import { test, expect } from "./tool-host";

test("long inputs scroll independently while run and result controls stay visible", async ({
  page,
  host,
}, testInfo) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await host.open(page);
  await prepareSampleInput(page, "Scrollable inputs");
  const run = page.getByRole("button", { name: "Run", exact: true });
  await expect(run).toBeInViewport();
  const input = page.getByRole("region", {
    name: "Config settings",
    exact: true,
  });
  const result = page.getByRole("region", {
    name: "Result content",
    exact: true,
  });
  expect(
    await input.evaluate((node) => node.scrollHeight > node.clientHeight),
  ).toBe(true);
  await input.focus();
  await page.keyboard.press("End");
  await expect
    .poll(() => input.evaluate((node) => node.scrollTop))
    .toBeGreaterThan(0);
  expect(await result.evaluate((node) => node.scrollTop)).toBe(0);
  await expect(run).toBeInViewport();
  await run.click();
  await expect(page.getByText("Contract evidence: complete.", { exact: false }).first()).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Inspect run", exact: true }),
  ).toBeInViewport();
  await expect(
    page.getByRole("img", { name: "Source", exact: true }),
  ).toBeVisible();
  for (const size of [
    { width: 1440, height: 900 },
    { width: 1024, height: 768 },
  ]) {
    await page.setViewportSize(size);
    await expect(run).toBeInViewport();
    expect(
      await page.evaluate(() => document.documentElement.scrollHeight),
    ).toBeLessThanOrEqual(size.height + 1);
    await page.screenshot({
      path: testInfo.outputPath(`panes-${size.width}.png`),
    });
  }
  for (const size of [
    { width: 320, height: 900 },
    { width: 1280, height: 500 },
  ]) {
    await page.setViewportSize(size);
    expect(
      await input.evaluate((node) => getComputedStyle(node).overflowY),
    ).toBe("visible");
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth),
    ).toBeLessThanOrEqual(size.width);
    await run.scrollIntoViewIfNeeded();
    await expect(run).toBeInViewport();
    await page.screenshot({
      path: testInfo.outputPath(`flow-${size.width}.png`),
      fullPage: true,
    });
  }
});
