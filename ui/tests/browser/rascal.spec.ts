import { test, expect } from "./tool-host";
import { readFile, readdir } from "node:fs/promises";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

test.use({ hostKind: "rascal", viewport: { width: 1440, height: 900 } });
test.skip(
  !process.env.WAFFLERY_RASCAL_BIN,
  "Opt in with an absolute WAFFLERY_RASCAL_BIN",
);
test("real Rascal runs offline and preserves browser evidence across restart", async ({
  page,
  host,
}, testInfo) => {
  test.setTimeout(60_000);
  page.setDefaultTimeout(10_000);
  const original = await readFile("tests/fixtures/panorama.png");
  await host.open(page);
  await page.getByRole("button", { name: /^Action / }).click();
  await page.getByRole("button", { name: /Heal a seam/ }).click();
  await page
    .locator("input[type=file]")
    .setInputFiles("tests/fixtures/panorama.png");
  await page
    .getByRole("combobox", { name: "Source image coordinates", exact: true })
    .selectOption("equirectangular");
  await page
    .getByLabel("Projection", { exact: true })
    .selectOption({ label: "equirectangular" });
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(
    page.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible({ timeout: 30_000 });
  await expect(
    page.getByRole("img", { name: "Source", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("img", { name: "Mask", exact: true }),
  ).toBeVisible();
  const maskRange = await page
    .getByRole("img", { name: "Mask", exact: true })
    .evaluate(async (element) => {
      const image = element as HTMLImageElement;
      await image.decode();
      const canvas = document.createElement("canvas");
      canvas.width = image.naturalWidth;
      canvas.height = image.naturalHeight;
      const context = canvas.getContext("2d")!;
      context.drawImage(image, 0, 0);
      const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
      let min = 255;
      let max = 0;
      for (let offset = 0; offset < pixels.length; offset += 4) {
        min = Math.min(min, pixels[offset]!);
        max = Math.max(max, pixels[offset]!);
      }
      return { min, max };
    });
  expect(maskRange.min).toBe(0);
  expect(maskRange.max).toBeGreaterThan(127);
  // Rascal's mask preview has different coordinates; the host must not invent alignment.
  await expect(
    page.getByRole("button", { name: "Wipe", exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Inspect run" }).click();
  const inspection = page.getByRole("dialog", {
    name: "Run details",
    exact: true,
  });
  await expect(inspection.getByLabel("Saved evidence")).toContainText(
    "equirectangular",
  );
  await inspection.getByRole("button", { name: "Recorded call" }).click();
  await expect(inspection.getByLabel("Saved evidence")).toContainText('"seam"');
  await inspection.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByRole("button", { name: "Use in your pipeline" }).click();
  const pipeline = page.getByRole("dialog", {
    name: "Use in your pipeline",
    exact: true,
  });
  await pipeline
    .getByRole("button", { name: "Evidence archive", exact: true })
    .click();
  await pipeline.getByLabel("Include image files").check();
  const downloading = page.waitForEvent("download");
  await pipeline
    .getByRole("button", { name: "Download evidence archive" })
    .click();
  const archive = testInfo.outputPath("rascal-evidence.tar.gz");
  await (await downloading).saveAs(archive);
  const extracted = spawnSync(
    "/usr/bin/tar",
    ["-xOzf", archive, "evidence/attempt.json"],
    { encoding: "utf8" },
  );
  expect(extracted.status).toBe(0);
  const evidence = JSON.parse(extracted.stdout);
  expect(evidence.result.spend.usd).toBe(0);
  expect(evidence.result.tool.name).toBe("rascal");
  expect(evidence.outputs.length).toBeGreaterThan(0);
  expect(evidence.preservation.contract).toBe("complete");
  const toolFiles = (await readdir(resolve(host.root, "../tool/runs"))).sort();
  await pipeline.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByRole("button", { name: "Examples", exact: true }).click();
  const examples = page.getByRole("dialog", { name: "Examples", exact: true });
  await examples
    .getByLabel("Example name", { exact: true })
    .fill("Real seam mask");
  await examples
    .getByRole("button", { name: "Save example", exact: true })
    .click();
  await examples.getByRole("button", { name: "Open Real seam mask" }).click();
  await expect(page.getByLabel("Projection", { exact: true })).toHaveValue(
    '"equirectangular"',
  );
  expect((await readdir(resolve(host.root, "../tool/runs"))).sort()).toEqual(
    toolFiles,
  );
  await host.stop();
  await host.start();
  await host.open(page);
  await page.getByRole("button", { name: "History", exact: true }).click();
  await page
    .getByRole("button", { name: /rascal \/ seam · completed/ })
    .click();
  await expect(
    page.getByRole("img", { name: "Mask", exact: true }),
  ).toBeVisible();
  await page.screenshot({
    path: testInfo.outputPath("real-rascal.png"),
    fullPage: true,
  });
  expect(await readFile("tests/fixtures/panorama.png")).toEqual(original);
  expect((await readdir(join(host.root, "../tool/runs"))).sort()).toEqual(
    toolFiles,
  );
});
