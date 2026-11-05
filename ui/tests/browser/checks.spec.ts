import { prepareSampleInput } from "./sample-input";
import { test, expect } from "./tool-host";
import { writeFile } from "node:fs/promises";
import { join } from "node:path";
import type { Page } from "@playwright/test";

// The sample peer's `checks` scenario adds "Add a checked band": an action that answers with a
// verdict (table-manners 0.5 draft, 2.4), an optional checking key (3.1) and labelled outputs
// for each try (10.2). Its `answer` and `tries` settings script what the sample check says.
test.use({ hostScenario: "checks" });

async function run(page: Page, label: string, settings: { answer?: string; tries?: number } = {}) {
  await prepareSampleInput(page, label);
  if (settings.answer) await page.getByLabel("Sample check answers", { exact: true }).selectOption({ label: settings.answer });
  if (settings.tries) await page.getByLabel("Tries", { exact: true }).fill(String(settings.tries));
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.getByText("Contract evidence: complete.", { exact: false }).first()).toBeVisible();
}

test("a missing optional key leaves the action ready and the result says it was not checked", async ({ page, host }) => {
  await writeFile(join(host.root, "..", "tool", "check-key-state"), "missing");
  await host.open(page, "/sample/checked_band");
  await page.getByRole("button", { name: "Tool setup", exact: true }).click();
  const drawer = page.getByRole("dialog", { name: "Tool setup", exact: true });
  await expect(drawer.getByText("Ready", { exact: true })).toBeVisible();
  await expect(drawer.getByText("EXAMPLE_CHECK_KEY", { exact: true })).toBeVisible();
  await expect(drawer.getByText("Missing", { exact: true })).toBeVisible();
  await expect(drawer.getByText("Needed for checking the result (optional)", { exact: true })).toBeVisible();
  await page.screenshot({ path: "test-results/checks-setup-optional.png" });
  await page.keyboard.press("Escape");
  await expect(drawer).not.toBeVisible();
  await expect(page.getByText("This activity needs setup.", { exact: false })).toHaveCount(0);
  // `check` is an ordinary optional yes-or-no setting, starting at the tool's default.
  const check = page.getByLabel("Check the result", { exact: true });
  await expect(check).toBeVisible();
  await expect(check.locator("option:checked")).toHaveText("Yes");
  const field = page.locator("label").filter({ has: check });
  await expect(field).toContainText("Optional");
  await expect(field.getByText("Check the result", { exact: true }).first()).toBeVisible();
  await run(page, "Unkeyed check");
  const verdict = page.getByRole("region", { name: "Check", exact: true });
  await expect(verdict).toContainText("Not checked: the sample check needs EXAMPLE_CHECK_KEY.");
  await expect(verdict).not.toContainText("Check passed");
  await page.screenshot({ path: "test-results/checks-unkeyed.png", fullPage: true });
});

test("a passing check says so near the result", async ({ page, host }) => {
  await host.open(page, "/sample/checked_band");
  await run(page, "Passing check");
  const verdict = page.getByRole("region", { name: "Check", exact: true });
  await expect(verdict).toContainText("Check passed");
  await expect(verdict).toContainText("Kept try 1 of 1.");
  await expect(verdict.getByRole("img")).toHaveCount(0);
  // One try: the outputs keep their roles' names.
  await expect(page.getByRole("button", { name: "Show Result", exact: true })).toBeVisible();
});

test("a failing check draws the places it still sees on the result", async ({ page, host }) => {
  await host.open(page, "/sample/checked_band");
  await run(page, "Failing check", { answer: "fail" });
  const verdict = page.getByRole("region", { name: "Check", exact: true });
  await expect(verdict).toContainText("The check still sees it");
  await expect(verdict).toContainText("Marked in 1 place on the result.");
  const marked = verdict.getByRole("img", { name: "Where the check still sees it · Result", exact: true });
  await expect(marked).toBeVisible();
  await expect(verdict.locator("svg polygon")).toHaveCount(1);
  await expect(verdict.getByRole("listitem").filter({ hasText: /^the band$/ })).toBeVisible();
  // The mark covers the place's fraction of the picture: [0.3, 0.38, 0.62, 0.62].
  const [picture, mark] = await Promise.all([marked.boundingBox(), verdict.locator("svg polygon").boundingBox()]);
  expect(Math.abs(mark!.x - (picture!.x + 0.3 * picture!.width))).toBeLessThan(3);
  expect(Math.abs(mark!.width - 0.32 * picture!.width)).toBeLessThan(4);
  await page.screenshot({ path: "test-results/checks-fail-desktop.png", fullPage: true });
  await page.setViewportSize({ width: 390, height: 844 });
  await expect(marked).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await verdict.scrollIntoViewIfNeeded();
  await page.screenshot({ path: "test-results/checks-fail-phone.png" });
});

test("three tries show by their labels, keep them after a restart, and try 1's mask opens", async ({ page, host }) => {
  await host.open(page, "/sample/checked_band");
  await run(page, "Three tries", { answer: "unsure", tries: 3 });
  const verdict = page.getByRole("region", { name: "Check", exact: true });
  await expect(verdict).toContainText("The check still sees it");
  await expect(verdict).toContainText("The check was not sure. Marked in 1 place on the result.");
  await expect(verdict).toContainText("Kept try 3 of 3. The check still sees the band in 1 place (unsure).");
  const show = page.getByRole("button", { name: "Show Try 3 · result", exact: true });
  await expect(show).toBeVisible();
  await show.click();
  const menu = page.getByRole("dialog", { name: "Choose show" });
  for (const k of [1, 2, 3]) for (const name of ["result", "preview", "mask"])
    await expect(menu.getByRole("button", { name: `Try ${k} · ${name}`, exact: true })).toBeVisible();
  // No internal role names reach the menu.
  await expect(menu).not.toContainText("try_");
  await page.screenshot({ path: "test-results/checks-tries-menu.png", fullPage: true });
  await menu.getByRole("button", { name: "Try 1 · mask", exact: true }).click();
  await expect(page.getByRole("button", { name: "Show Try 1 · mask", exact: true })).toBeVisible();
  const mask = page.getByRole("img", { name: "Try 1 · mask", exact: true });
  await expect(mask).toBeVisible();
  // Try 1's mask: grey, white only on the band's rows.
  const grey = async () => mask.evaluate(async (element) => {
    const image = element as HTMLImageElement;
    await image.decode();
    const canvas = document.createElement("canvas");
    canvas.width = image.naturalWidth; canvas.height = image.naturalHeight;
    const context = canvas.getContext("2d")!;
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    let white = 0;
    for (let i = 0; i < pixels.length; i += 4) {
      if (pixels[i] !== pixels[i + 1] || pixels[i] !== pixels[i + 2]) return -1;
      if (pixels[i] === 255) white++;
    }
    return white / (canvas.width * canvas.height);
  });
  const covered = await grey();
  expect(covered).toBeGreaterThan(0.1);
  expect(covered).toBeLessThan(0.3);
  // A saved run, reopened after the host restarts, keeps the same names and the same verdict.
  const address = new URL(page.url()).pathname;
  await host.stop();
  await host.start();
  await host.open(page, address);
  await expect(page.getByRole("region", { name: "Check", exact: true })).toContainText("The check still sees it");
  await expect(page.getByRole("region", { name: "Check", exact: true }).locator("svg polygon")).toHaveCount(1);
  await page.getByRole("button", { name: "Show Try 3 · result", exact: true }).click();
  await page.getByRole("dialog", { name: "Choose show" }).getByRole("button", { name: "Try 1 · mask", exact: true }).click();
  await expect(page.getByRole("img", { name: "Try 1 · mask", exact: true })).toBeVisible();
  expect(await grey()).toBe(covered);
});
