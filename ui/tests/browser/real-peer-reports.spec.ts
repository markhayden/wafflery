import { test, expect } from "./tool-host";
import { chooseReport } from "./report-fixture";

test.use({ hostKind: "squint", hostScenario: "references", viewport: { width: 1440, height: 1000 } });
test.skip(!process.env.WAFFLERY_RASCAL_BIN || !process.env.WAFFLERY_SQUINT_BIN,
  "Select the approved contract 0.4.0 Rascal and Squint testkit binaries");

for (const tool of ["rascal", "squint"] as const) {
  test(`real ${tool} reports use saved evidence after draft edits and host restart`, async ({ page, host }) => {
    test.setTimeout(60_000);
    page.setDefaultTimeout(10_000);
    const errors: string[] = [];
    page.on("pageerror", error => errors.push(error.message));
    page.on("console", message => { if (message.type() === "error") errors.push(message.text()); });
    let token = "";
    async function open() {
      const connected = page.waitForResponse(response => new URL(response.url()).pathname === "/api/session");
      await host.open(page);
      token = (await (await connected).json()).token;
    }
    async function api(path: string) {
      const response = await page.request.get(`${host.origin}${path}`, {
        headers: { authorization: `Bearer ${token}`, origin: host.origin },
      });
      expect(response.ok()).toBe(true);
      return response.json();
    }
    await open();
    if (tool === "rascal") {
      await page.getByRole("button", { name: "Tool squint", exact: true }).click();
      const toolRow=page.getByRole("dialog",{name:"Choose tool"}).getByRole("button",{name:/^rascal /});
      await expect(toolRow.getByText(/^Version /)).toBeVisible();
      await toolRow.click();
    }
    await page.getByRole("button", { name: /^Action / }).click();
    await page.getByRole("button", { name: tool === "rascal" ? /Heal a seam/ : /Reference consistency Image/ }).click();
    if (tool === "rascal") {
      await page.locator("input[type=file]").setInputFiles("tests/fixtures/panorama.png");
      await page.getByRole("combobox", { name: "Source image coordinates", exact: true }).selectOption("equirectangular");
      await page.getByLabel("Projection", { exact: true }).selectOption({ label: "equirectangular" });
    } else {
      await page.getByRole("region", { name: "Candidate image", exact: true })
        .locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
      await page.getByRole("region", { name: "References (reference image)", exact: true })
        .locator("input[type=file]").setInputFiles([
          "tests/fixtures/panorama.png", "tests/fixtures/sample.png", "tests/fixtures/panorama.png",
        ]);
    }
    const submitted = page.waitForResponse(response => new URL(response.url()).pathname === "/api/attempts" && response.request().method() === "POST");
    await page.getByRole("button", { name: "Run", exact: true }).click();
    const { id } = await (await submitted).json();
    await expect.poll(async () => (await api(`/api/attempts/${id}`)).preservation.contract).toBe("complete");
    const saved = await api(`/api/attempts/${id}`);
    expect(saved.execution).toBe("completed");
    if (tool === "squint") {
      const references = saved.inputs.references;
      expect(references).toHaveLength(3);
      expect(references[0].asset.id).toBe(references[2].asset.id);
      expect(references[1].asset.id).not.toBe(references[0].asset.id);
      expect(saved.result.images.in.filter((image: { slot: string }) => image.slot === "references")
        .map((image: { image: { file_sha256: string } }) => image.image.file_sha256))
        .toEqual(references.map((entry: { asset: { sha256: string } }) => entry.asset.sha256));
      await page.getByRole("region", { name: "References (reference image)", exact: true })
        .getByRole("button", { name: "Remove image 1", exact: true }).click();
    } else {
      await page.getByLabel("Projection", { exact: true }).selectOption({ label: "flat" });
    }
    const reports = tool === "rascal" ? [["dash", "Live viewer"], ["page", "Document"]] : [["report", "Live viewer"], ["export", "Document"]];
    async function inspect(name: string, kind: string) {
      await page.getByRole("button", { name: "Inspect run", exact: true }).click();
      await page.getByRole("button", { name: "Reports", exact: true }).click();
      await chooseReport(page, name, kind);
      await page.getByRole("button", { name: "Open report", exact: true }).click();
      const report = page.frameLocator('iframe[title="Report viewer"]').frameLocator('iframe[title="Tool report"]');
      await expect(report.getByRole("heading", { name: tool === "rascal" ? /rascal run page/ : "squint report", level: 1 })).toBeVisible();
      if (tool === "squint") {
        await expect(report.locator("#status")).toContainText("reference-consistency");
        // Read the rendered legend: visible content alone missed the original CSP defect.
        const swatches = report.locator(".legend i");
        await expect(swatches).toHaveCount(4);
        const colors = await swatches.evaluateAll(nodes => nodes.map(node => getComputedStyle(node).backgroundColor));
        expect(colors.every(color => color !== "rgba(0, 0, 0, 0)" && color !== "transparent")).toBe(true);
        expect(new Set(colors).size).toBe(4);
      }
      else await expect(report.getByRole("heading", { name: "Steps", exact: true })).toBeVisible();
      const { view } = await api(`/api/attempts/${id}/views`);
      expect(view.phase).toBe("ready");
      expect(view.error).toBeNull();
      // Do not capture screenshots here: Playwright's WebKit screenshot preparation injects
      // an inline `body {}` stylesheet into every frame, violating the tool's strict CSP.
      // Keep console collection unfiltered and verify rendered styles directly instead.
      await page.getByRole("dialog", { name: "Run details", exact: true }).getByRole("button", { name: "Close", exact: true }).click();
      await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("exited");
      expect(await api(`/api/attempts/${id}`)).toEqual(saved);
    }
    for (const [name, kind] of reports) await inspect(name!, kind!);
    await page.goto("about:blank");
    await host.stop();
    await host.start();
    await open();
    await page.getByRole("button", { name: "History", exact: true }).click();
    await page.getByRole("button", { name: new RegExp(`${tool} / ${saved.request.action} · completed`) }).click();
    expect(await api(`/api/attempts/${id}`)).toEqual(saved);
    await inspect(reports[1]![0]!, reports[1]![1]!);
    expect(errors).toEqual([]);
  });
}
