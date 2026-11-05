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
test("a panorama's seam can be turned to the middle, and a flat pair offers no turn", async ({ page }) => {
  await page.goto("http://127.0.0.1:6006/iframe.html?id=components-imagecompare--panorama&viewMode=story");
  const centre = page.getByRole("button", { name: "Centre the seam" });
  await expect(centre).toHaveAttribute("aria-pressed", "false");
  await page.getByRole("button", { name: "Side by side" }).click();
  // The story's panorama has a red marker at each edge; turned, they meet in the middle.
  const middleRed = () => page.getByRole("img", { name: "Result" }).evaluate(async (node) => {
    const img = node as HTMLImageElement;
    await img.decode();
    const canvas = document.createElement("canvas");
    canvas.width = img.naturalWidth; canvas.height = img.naturalHeight;
    const c = canvas.getContext("2d")!;
    c.drawImage(img, 0, 0);
    const [r, g, b] = c.getImageData(img.naturalWidth / 2, img.naturalHeight / 2, 1, 1).data;
    return r! > 200 && g! < 120 && b! < 120;
  });
  expect(await middleRed()).toBe(false);
  await centre.click();
  await expect(centre).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByText(/the seam, where the panorama's edges meet, runs down the middle/)).toBeVisible();
  await expect.poll(middleRed).toBe(true);
  await centre.click();
  await expect.poll(middleRed).toBe(false);
  await page.goto("http://127.0.0.1:6006/iframe.html?id=components-imagecompare--wipe&viewMode=story");
  await expect(page.getByRole("slider", { name: "Before and after divider" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Centre the seam" })).toHaveCount(0);
});
test("a portrait image fits a short window whole in every mode", async ({ page }) => {
  await page.setViewportSize({ width: 1600, height: 700 });
  await page.goto("http://127.0.0.1:6006/iframe.html?id=components-imagecompare--portrait&viewMode=story");
  const limit = 700 * 0.65 + 1;
  const wipe = page.getByRole("slider", { name: "Before and after divider" }).locator("..");
  await expect(wipe).toBeVisible();
  const box = (await wipe.boundingBox())!;
  expect(box.height).toBeLessThanOrEqual(limit);
  expect(box.width / box.height).toBeCloseTo(768 / 1376, 2);
  await page.getByRole("button", { name: "Changes" }).click();
  const changes = (await page.getByRole("img", { name: /Changes from/ }).locator("..").boundingBox())!;
  expect(changes.height).toBeLessThanOrEqual(limit);
  await page.getByRole("button", { name: "Side by side" }).click();
  for (const name of ["Source", "Result"]) {
    const img = (await page.getByRole("img", { name, exact: true }).boundingBox())!;
    expect(img.height).toBeLessThanOrEqual(limit);
  }
});
