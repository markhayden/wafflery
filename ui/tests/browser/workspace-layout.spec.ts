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
  // The panels fill the window exactly at every desktop height a laptop has: the page never
  // scrolls into the space below the shorter panel, and each panel scrolls its own content.
  for (const size of [
    { width: 1440, height: 900 },
    { width: 1280, height: 720 },
    { width: 1024, height: 768 },
    { width: 1024, height: 640 },
    { width: 1280, height: 560 },
  ]) {
    await page.setViewportSize(size);
    await expect(run).toBeInViewport();
    await expect(page.getByRole("button", { name: "Inspect run", exact: true })).toBeInViewport();
    expect(
      await page.evaluate(() => document.documentElement.scrollHeight),
    ).toBeLessThanOrEqual(size.height + 1);
    const [config, outcome] = await Promise.all([
      page.locator("form", { has: input }).boundingBox(),
      page.getByRole("region", { name: "Result", exact: true }).boundingBox(),
    ]);
    expect(config && outcome).toBeTruthy();
    // Both panels end at the same line, close to the bottom of the window.
    expect(Math.abs(config!.y + config!.height - (outcome!.y + outcome!.height))).toBeLessThanOrEqual(1);
    expect(size.height - (config!.y + config!.height)).toBeLessThanOrEqual(48);
    expect(await input.evaluate((node) => getComputedStyle(node).overflowY)).toBe("auto");
    await page.screenshot({
      path: testInfo.outputPath(`panes-${size.width}.png`),
    });
  }
  // A phone stacks the panels, and a window too short for both panels' bars scrolls the page.
  for (const size of [
    { width: 320, height: 900 },
    { width: 390, height: 844 },
    { width: 1280, height: 500 },
  ]) {
    await page.setViewportSize(size);
    expect(
      await input.evaluate((node) => getComputedStyle(node).overflowY),
    ).toBe("visible");
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth),
    ).toBeLessThanOrEqual(size.width);
    // The page ends with the panels, side by side ending on one line: no empty space to scroll
    // into below a shorter one.
    const bottoms = await Promise.all([page.locator("form", { has: input }), page.getByRole("region", { name: "Result", exact: true })]
      .map(panel => panel.evaluate(node => node.getBoundingClientRect().bottom + scrollY)));
    const end = await page.evaluate(() => document.documentElement.scrollHeight);
    expect(end - Math.max(...bottoms)).toBeLessThanOrEqual(64);
    if (size.width > 960) expect(Math.abs(bottoms[0]! - bottoms[1]!)).toBeLessThanOrEqual(1);
    await run.scrollIntoViewIfNeeded();
    await expect(run).toBeInViewport();
    await page.screenshot({
      path: testInfo.outputPath(`flow-${size.width}.png`),
      fullPage: true,
    });
  }
});
