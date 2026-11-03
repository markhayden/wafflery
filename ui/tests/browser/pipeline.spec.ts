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
  await drawer.getByRole("button",{name:"Recorded MCP call",exact:true}).click();
  await expect(
    drawer.getByLabel("Pipeline evidence", { exact: true }),
  ).toContainText("Export this run");
  await expect(
    drawer.getByLabel("Pipeline evidence", { exact: true }),
  ).not.toContainText("Do not export this draft");
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
  await drawer.getByRole("button", { name: "Copy recorded call" }).click();
  expect(
    await page.evaluate(() => (window as unknown as { copied: string }).copied),
  ).toContain("Export this run");
  await drawer
    .getByRole("button", { name: "Evidence archive", exact: true })
    .click();
  await drawer.getByLabel("Include image files").uncheck();
  await expect(drawer.getByText(/References only:/)).toBeVisible();
  const downloadEvent = page.waitForEvent("download");
  await drawer
    .getByRole("button", { name: "Download evidence archive" })
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
