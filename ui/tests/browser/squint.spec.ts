import { test, expect } from "./tool-host";
import { readFile, readdir } from "node:fs/promises";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

test.use({ hostKind: "squint", viewport: { width: 1440, height: 900 } });
test.skip(
  !process.env.WAFFLERY_SQUINT_BIN || !process.env.WAFFLERY_RASCAL_BIN,
  "Select stable contract 0.5.0 Rascal and Squint testkit executables explicitly",
);

test("Squint scripted evidence survives export, examples, tool switching and restart", async ({
  page,
  host,
}, testInfo) => {
  test.setTimeout(60_000);
  const original = await readFile("tests/fixtures/sample.png");
  await host.open(page);
  await page.getByRole("button", { name: /^Action / }).click();
  await page.getByRole("button", { name: /^Source fidelity / }).click();
  await page
    .locator("input[type=file]")
    .nth(0)
    .setInputFiles("tests/fixtures/sample.png");
  await page
    .locator("input[type=file]")
    .nth(1)
    .setInputFiles("tests/fixtures/sample.png");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(
    page.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible();
  await expect(page.getByLabel("Result data", { exact: true })).toContainText(
    '"verdict": "pass"',
  );
  await expect(
    page.getByRole("region", { name: "Image comparison", exact: true }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Inspect run" }).click();
  const inspector = page.getByRole("dialog", {
    name: "Run details",
    exact: true,
  });
  await inspector.getByRole("button", { name: "Recorded call" }).click();
  await expect(inspector.getByLabel("Saved evidence")).toContainText(
    '"source_fidelity"',
  );
  await inspector.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByRole("button", { name: "Use in your pipeline" }).click();
  const pipeline = page.getByRole("dialog", {
    name: "Use in your pipeline",
    exact: true,
  });
  await pipeline.getByLabel("Include image files").check();
  const downloading = page.waitForEvent("download");
  await pipeline
    .getByRole("button", { name: "Download archive" })
    .click();
  const archive = testInfo.outputPath("squint-evidence.tar.gz");
  await (await downloading).saveAs(archive);
  const extracted = spawnSync(
    "/usr/bin/tar",
    ["-xOzf", archive, "evidence/attempt.json"],
    { encoding: "utf8" },
  );
  expect(extracted.status).toBe(0);
  const evidence = JSON.parse(extracted.stdout);
  expect(evidence.result.tool.name).toBe("squint");
  expect(evidence.result.body.verdict).toBe("pass");
  expect(evidence.result.spend.usd).toBe(0);
  expect(evidence.preservation.contract).toBe("complete");
  expect(evidence.inputs.candidate[0].asset.sha256).toBe(
    evidence.inputs.source[0].asset.sha256,
  );
  await pipeline.getByRole("button", { name: "Close", exact: true }).click();
  const runNames = await readdir(resolve(host.root, "../squint/runs"));
  await page.getByRole("button", { name: "Examples", exact: true }).click();
  const examples = page.getByRole("dialog", { name: "Examples", exact: true });
  await examples
    .getByLabel("Example name", { exact: true })
    .fill("Scripted source check");
  await examples
    .getByRole("button", { name: "Save example", exact: true })
    .click();
  await examples
    .getByRole("button", { name: "Open Scripted source check" })
    .click();
  await expect(
    page.getByText("Example loaded: Scripted source check", { exact: true }),
  ).toBeVisible();
  expect(await readdir(resolve(host.root, "../squint/runs"))).toEqual(runNames);
  await page.getByRole("button", { name: "Tool squint", exact: true }).click();
  const toolRow=page.getByRole("dialog",{name:"Choose tool"}).getByRole("button",{name:/^rascal /});
      await expect(toolRow.getByText(/^Version /)).toBeVisible();
      await toolRow.click();
  await expect(page.getByLabel("Result data", { exact: true })).toHaveCount(0);
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
    page.getByRole("img", { name: "Preview", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible();
  await host.stop();
  await host.start();
  await host.open(page);
  await page.getByRole("button", { name: "History", exact: true }).click();
  await page
    .getByRole("button", { name: /squint \/ source_fidelity · completed/ })
    .click();
  await expect(
    page.getByRole("button", { name: "Tool squint", exact: true }),
  ).toBeVisible();
  await expect(page.getByLabel("Result data", { exact: true })).toContainText(
    '"verdict": "pass"',
  );
  await expect(
    page.getByRole("img", { name: /^(Mask|Preview)$/ }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("dialog", { name: "History", exact: true }),
  ).not.toBeVisible();
  await page.screenshot({
    path: testInfo.outputPath("two-tool-squint.png"),
    fullPage: true,
  });
  expect(await readdir(join(host.root, "../squint/runs"))).toEqual(runNames);
  expect(await readFile("tests/fixtures/sample.png")).toEqual(original);
});
