import { prepareSampleInput } from "./sample-input";
import { test, expect } from "./tool-host";
import { spawnSync } from "node:child_process";
test("pipeline drawer copies and downloads the selected saved evidence", async ({
  page,
  host,
}, testInfo) => {
  await host.open(page);
  await prepareSampleInput(page, "Export this run");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(
    page.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible();
  await page
    .getByLabel("Label", { exact: true })
    .fill("Do not export this draft");
  await page
    .getByRole("button", { name: "Use in your pipeline", exact: true })
    .click();
  const drawer = page.getByRole("dialog", {
    name: "Use in your pipeline",
    exact: true,
  });
  // The recorded call now lives in Run details, under Advanced.
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Inspect run", exact: true }).click();
  const details = page.getByRole("dialog", { name: "Run details", exact: true });
  await details.getByRole("button", { name: "Advanced", exact: true }).click();
  await details.getByRole("button", { name: "Recorded call", exact: true }).click();
  await expect(details.getByLabel("Saved evidence", { exact: true })).toContainText("Export this run");
  await expect(details.getByLabel("Saved evidence", { exact: true })).not.toContainText("Do not export this draft");
  await page.evaluate(() => {
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: async (text: string) => {
          (window as unknown as { copied: string }).copied = text;
        },
      },
    });
  });
  await details.getByRole("button", { name: "Copy this record" }).click();
  expect(
    await page.evaluate(() => (window as unknown as { copied: string }).copied),
  ).toContain("Export this run");
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Use in your pipeline", exact: true }).click();
  await drawer.getByLabel("Include image files").uncheck();
  await expect(drawer.getByText(/References only:/)).toBeVisible();
  const downloadEvent = page.waitForEvent("download");
  await drawer
    .getByRole("button", { name: "Download archive" })
    .click();
  const download = await downloadEvent;
  const path = testInfo.outputPath("evidence.tar.gz");
  await download.saveAs(path);
  const manifest = spawnSync("/usr/bin/tar", ["-xOzf", path, "manifest.json"], {
    encoding: "utf8",
  });
  expect(manifest.status).toBe(0);
  expect(JSON.parse(manifest.stdout).images_included).toBe(false);
  const evidence = spawnSync(
    "/usr/bin/tar",
    ["-xOzf", path, "evidence/attempt.json"],
    { encoding: "utf8" },
  );
  expect(evidence.status).toBe(0);
  expect(JSON.parse(evidence.stdout).request.params.label).toBe(
    "Export this run",
  );
});
