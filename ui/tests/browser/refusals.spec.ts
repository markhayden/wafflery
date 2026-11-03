import { test, expect } from "./tool-host";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

async function prepare(page: import("@playwright/test").Page, label: string) {
  await page.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
  const source = page.getByRole("region", { name:"Source image",exact: true });
  await expect(source).toHaveAttribute("aria-busy", "false");
  await expect(source.getByRole("img", { name: "Selected image 1" })).toBeVisible();
  const input = page.getByLabel("Label", { exact: true });
  await input.click();
  await input.fill(label);
  await expect(input).toHaveValue(label);
}

function submissions(page: import("@playwright/test").Page) {
  const ids: string[] = [];
  page.on("request", request => {
    if (request.method() === "POST" && new URL(request.url()).pathname === "/api/attempts")
      ids.push(request.postDataJSON().id);
  });
  return ids;
}

test.describe("no-run refusals", () => {
  test.use({ hostScenario: "refusals" });
  for (const label of ["refuse plain", "refuse structured"]) {
    test(`${label} shows no run and each deliberate attempt has a new identity`, async ({ page, host }, testInfo) => {
      await host.open(page);
      await prepare(page, label);
      const ids = submissions(page);
      const run = page.getByRole("button", { name: "Run", exact: true });
      const result = page.getByRole("region", { name: "Result content" });
      await run.click();
      await expect(result.getByText("No run created.", { exact: true })).toBeVisible();
      await expect(result).toContainText("No spend was incurred");
      await expect(result.getByText("Correct the input", { exact: true })).toBeVisible();
      await result.scrollIntoViewIfNeeded();
      await result.screenshot({ path: testInfo.outputPath("no-run-refusal.png") });
      await expect(result).not.toContainText("Contract evidence:");
      await expect(run).toBeEnabled();
      // An explicit retry is a new attempt even when inputs are unchanged.
      await run.click();
      await expect.poll(() => ids.length).toBe(2);
      await expect(page.getByText(`Saved attempt · ${ids[1]!.slice(0, 8)}`, { exact: true })).toBeVisible();
      await expect(run).toBeEnabled();
      await page.getByLabel("Label", { exact: true }).fill("Corrected input");
      await run.click();
      await expect(result).toContainText("Contract evidence: complete.");
      expect(new Set(ids).size).toBe(3);
      const calls = await readFile(join(host.root, "..", "tool", "calls.ndjson"), "utf8");
      expect(calls.trim().split("\n")).toHaveLength(1);
    });
  }
});

test("a lost HTTP acknowledgement retains identity without replay; a later deliberate run is new", async ({ page, host }) => {
  await host.open(page);
  await prepare(page, "One execution despite a lost acknowledgement");
  const ids = submissions(page);
  let loseReply = true;
  await page.route("**/api/attempts", async route => {
    if (route.request().method() !== "POST") return route.continue();
    const response = await route.fetch();
    if (loseReply) { loseReply = false; await route.abort("failed"); }
    else await route.fulfill({ response });
  });
  try {
    const run = page.getByRole("button", { name: "Run", exact: true });
    await run.click();
    await expect(page.getByRole("region", { name: "Result content" })).toContainText("Contract evidence: complete.");
    await expect(run).toBeEnabled();
    expect(ids).toHaveLength(1);
    await run.click();
    await expect.poll(() => ids.length).toBe(2);
    await expect(run).toBeEnabled();
    expect(ids[1]).toBe(ids[0]);
    await run.click();
    await expect.poll(() => ids.length).toBe(3);
    await expect(page.getByText(`Saved attempt · ${ids[2]!.slice(0, 8)}`, { exact: true })).toBeVisible();
    await expect(page.getByRole("region", { name: "Result content" })).toContainText("Contract evidence: complete.");
    expect(ids[2]).not.toBe(ids[0]);
    const calls = await readFile(join(host.root, "..", "tool", "calls.ndjson"), "utf8");
    expect(calls.trim().split("\n")).toHaveLength(2);
  } finally { await page.unrouteAll({ behavior: "wait" }); }
});

test("a failed snapshot after an acknowledged submission does not reuse that attempt", async ({ page, host }) => {
  await host.open(page);
  await prepare(page, "Acknowledged before refresh failure");
  const ids = submissions(page);
  let blockSnapshot = false;
  await page.route("**/api/state", route => blockSnapshot
    ? route.fulfill({ status: 503, contentType: "application/json", body: JSON.stringify({ error: { message: "Synthetic refresh failure" } }) })
    : route.continue());
  await page.route("**/api/attempts", async route => {
    if (route.request().method() !== "POST") return route.continue();
    const response = await route.fetch();
    if (ids.length === 1) blockSnapshot = true;
    await route.fulfill({ response });
  });
  try {
    const run = page.getByRole("button", { name: "Run", exact: true });
    await run.click();
    await expect(page.getByRole("alert").filter({ hasText: "Synthetic refresh failure" })).toBeVisible();
    blockSnapshot = false;
    await page.getByRole("button", { name: "Tool setup", exact: true }).click();
    const drawer = page.getByRole("dialog", { name: "Tool setup", exact: true });
    await expect(drawer).toContainText("Uncapped");
    await drawer.getByRole("button", { name: "Refresh", exact: true }).click();
    await expect(drawer.getByRole("button", { name: "Refresh", exact: true })).toBeEnabled();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("region", { name: "Result content" })).toContainText("Contract evidence: complete.");
    await expect(run).toBeEnabled();
    await run.click();
    await expect.poll(() => ids.length).toBe(2);
    expect(ids[1]).not.toBe(ids[0]);
    await expect(page.getByText(`Saved attempt · ${ids[1]!.slice(0, 8)}`, { exact: true })).toBeVisible();
    await expect(page.getByRole("region", { name: "Result content" })).toContainText("Contract evidence: complete.");
  } finally { await page.unrouteAll({ behavior: "wait" }); }
});
