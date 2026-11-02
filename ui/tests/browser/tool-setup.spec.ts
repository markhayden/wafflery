import { test, expect } from "./tool-host";
import { writeFile } from "node:fs/promises";
import { join } from "node:path";

test("setup shows reported allowances and never promotes a credential name to readiness", async ({ page }) => {
  await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-toolsetup--capped&viewMode=story");
  await expect(page.getByRole("region", { name: "Session spending" })).toContainText("$1.80");
  await expect(page.getByText(/final charge is unknown/)).toContainText("$0.04");
  await expect(page.getByText("EXAMPLE_API_KEY", { exact: true })).toBeVisible();
  await expect(page.getByText("Present · not validated", { exact: true })).toBeVisible();
  await expect(page.getByText(/A present credential has not necessarily been validated/)).toBeVisible();
  await expect(page.getByText(/Paid execution needs/)).toContainText("not enabled yet");
  for (const width of [320, 768, 1024, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    await expect(page.getByRole("button", { name: "Refresh" })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  }
  await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-toolsetup--exhausted&viewMode=story");
  await expect(page.getByRole("region", { name: "Session spending" })).toContainText("$0.00");
  await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-toolsetup--unavailable&viewMode=story");
  await expect(page.getByRole("alert")).toContainText("disconnected");
  await expect(page.getByRole("region", { name: "Session spending" })).not.toContainText("$0.00");
});

test("live setup refresh preserves drafts and clears an unavailable allowance without running anything", async ({ page, host }) => {
  await host.open(page);
  const label = page.getByLabel("Label *", { exact: true });
  await label.click();
  await expect(label).toBeFocused();
  await label.fill("Keep this draft");
  await expect(label).toHaveValue("Keep this draft");
  let dispatched = 0;
  page.on("request", request => { if (request.method() === "POST" && new URL(request.url()).pathname === "/api/attempts") dispatched++; });
  await page.getByRole("button", { name: "Tool setup", exact: true }).click();
  const drawer = page.getByRole("dialog", { name: "Tool setup", exact: true });
  await expect(drawer.getByRole("region", { name: "Session spending" })).toContainText("Uncapped");
  await expect(drawer).toContainText("No setup requirements are declared");
  await drawer.getByText("Session identity", { exact: true }).click();
  const before = await drawer.locator("code").innerText();
  await drawer.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(drawer.getByRole("region", { name: "Session spending" })).toContainText("Uncapped");
  await expect(drawer.locator("code")).toHaveText(before);
  await page.route("**/api/tools/sample/session", route => route.fulfill({ status: 503, contentType: "application/json", body: JSON.stringify({error: {message: "Session temporarily unavailable"}}) }));
  await drawer.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(drawer.getByRole("alert")).toContainText("Session temporarily unavailable");
  await expect(drawer).not.toContainText("Uncapped");
  await expect(drawer).not.toContainText("$0.00");
  await page.unroute("**/api/tools/sample/session");
  await drawer.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(drawer).toContainText("Uncapped");
  await page.screenshot({ path: "test-results/tool-setup-live.png" });
  await page.keyboard.press("Escape");
  await expect(drawer).not.toBeVisible();
  await expect(page.getByRole("button", { name: "Tool setup", exact: true })).toBeFocused();
  await expect(page.getByLabel("Label *", { exact: true })).toHaveValue("Keep this draft");
  expect(dispatched).toBe(0);
});

test.describe("paid action setup", () => {
  test.use({ hostScenario: "paid" });
  test("opening setup never enables paid execution", async ({ page, host }) => {
    await host.open(page);
    await page.getByRole("button", { name: "Tool setup", exact: true }).click();
    await expect(page.getByRole("dialog")).toContainText("It is not enabled yet");
    await page.keyboard.press("Escape");
    await expect(page.getByRole("button", { name: "Run", exact: true })).toBeDisabled();
  });
});

test("an unavailable tool never replaces setup with a saved-run inspector", async ({ page, context, host }) => {
  await host.open(page);
  await page.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
  await page.getByLabel("Label *", { exact: true }).fill("Saved before setup");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.getByText("Contract evidence: complete.", { exact: false }).first()).toBeVisible();
  await page.getByRole("button", { name: "Tool setup", exact: true }).click();
  const drawer = page.getByRole("dialog", { name: "Tool setup", exact: true });
  await expect(drawer).toContainText("Uncapped");
  await page.route("**/api/state", async route => {
    const response = await route.fetch();
    const value = await response.json();
    value.tools = value.tools.map((tool: {id: string}) => ({ ...tool, declaration: null, error: "Tool description is unavailable" }));
    await route.fulfill({ response, json: value });
  });
  const other = await context.newPage();
  await host.open(other);
  await other.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
  await other.getByLabel("Label *", { exact: true }).fill("Trigger a state update");
  await other.getByRole("button", { name: "Run", exact: true }).click();
  await expect(drawer).toContainText("Setup is unavailable because this tool or action is no longer available.");
  await expect(drawer.getByRole("navigation", { name: "Run evidence" })).not.toBeVisible();
  await expect(drawer).not.toContainText("Uncapped");
  // Drain intercepted state reads before the host fixture shuts down.
  await page.unrouteAll({ behavior: "wait" });
  await other.close();
});


test("setup explains every requirement state and kind", async ({ page }, testInfo) => {
  await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-toolsetup--requirements&viewMode=story");
  const setup = page.getByRole("region", { name: "Provider setup" });
  for (const text of ["Setup may be needed", "Missing", "Present · not validated", "Validated", "Invalid", "Unknown", "Configuration", "Credential", "Environment variable", "Model", "File", "Dependency"]) {
    await expect(setup.getByText(text, { exact: true }).first()).toBeVisible();
  }
  await expect(setup).toContainText("Choose a configuration file outside the playground.");
  await page.screenshot({ path: testInfo.outputPath("readiness-requirements.png"), fullPage: true });
  await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-toolsetup--unavailable-action&viewMode=story");
  await expect(setup.getByText("Unavailable", { exact: true })).toBeVisible();
  await expect(setup).toContainText("This build does not include the backend.");
});

test.describe("live readiness", () => {
  test.use({ hostScenario: "readiness" });
  test("refresh changes readiness and revision while retaining the selected action and draft", async ({ page, host }) => {
    await host.open(page);
    await page.getByRole("button", { name: "Action Add a test band" }).click();
    await page.getByRole("button", { name: /Price a scripted call Parameters/ }).click();
    await page.getByLabel("label *", { exact: true }).fill("Keep this paid draft");
    let attempts = 0;
    page.on("request", request => { if (request.method() === "POST" && new URL(request.url()).pathname === "/api/attempts") attempts++; });
    const opening = page.waitForResponse(response => new URL(response.url()).pathname === "/api/tools/sample/refresh");
    await page.getByRole("button", { name: "Tool setup", exact: true }).click();
    let previous = (await (await opening).json()).revision;
    const drawer = page.getByRole("dialog", { name: "Tool setup", exact: true });
    await expect(drawer.getByText("Present · not validated", { exact: true })).toBeVisible();
    for (const [state, label] of [["valid", "Validated"], ["invalid", "Invalid"], ["missing", "Missing"], ["unknown", "Unknown"]] as const) {
      await writeFile(join(host.root, "..", "tool", "credential-state"), state);
      const response = page.waitForResponse(response => new URL(response.url()).pathname === "/api/tools/sample/refresh");
      await drawer.getByRole("button", { name: "Refresh", exact: true }).click();
      const current = (await (await response).json()).revision;
      expect(current).not.toBe(previous); previous = current;
      await expect(drawer.getByText(label, { exact: true })).toBeVisible();
      await expect(page.getByLabel("label *", { exact: true })).toHaveValue("Keep this paid draft");
    }
    await page.keyboard.press("Escape");
    await expect(page.getByRole("button", { name: "Action Price a scripted call" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Run", exact: true })).toBeDisabled();
    expect(attempts).toBe(0);
  });
  test("a delayed setup response cannot restore stale readiness after another tab refreshes", async ({ page, context, host }) => {
    await host.open(page);
    await page.getByRole("button", { name: "Action Add a test band" }).click();
    await page.getByRole("button", { name: /Price a scripted call Parameters/ }).click();
    await page.getByRole("button", { name: "Tool setup", exact: true }).click();
    const drawer = page.getByRole("dialog", { name: "Tool setup", exact: true });
    await expect(drawer.getByText("Present · not validated", { exact: true })).toBeVisible();
    await expect(drawer.getByRole("button", { name: "Refresh", exact: true })).toBeEnabled();
    let release: () => void = () => {};
    let held: () => void = () => {};
    const gate = new Promise<void>(resolve => { release = resolve; });
    const captured = new Promise<void>(resolve => { held = resolve; });
    await page.route("**/api/tools/sample/refresh", async route => {
      const response = await route.fetch(); held(); await gate; await route.fulfill({ response });
    });
    const other = await context.newPage();
    try {
      await drawer.getByRole("button", { name: "Refresh", exact: true }).click();
      await captured;
      await writeFile(join(host.root, "..", "tool", "credential-state"), "valid");
      await host.open(other);
      await other.getByRole("button", { name: "Action Add a test band" }).click();
      await other.getByRole("button", { name: /Price a scripted call Parameters/ }).click();
      await other.getByRole("button", { name: "Tool setup", exact: true }).click();
      await expect(other.getByRole("dialog").getByText("Validated", { exact: true })).toBeVisible();
      await expect(drawer.getByText("Validated", { exact: true })).toBeVisible();
      release();
      await expect(drawer.getByRole("button", { name: "Refresh", exact: true })).toBeEnabled();
      await expect(drawer.getByText("Validated", { exact: true })).toBeVisible();
      await expect(drawer.getByText("Present · not validated", { exact: true })).not.toBeVisible();
    } finally {
      release(); await page.unrouteAll({ behavior: "wait" }); await other.close();
    }
  });
  test("optional setup refuses only the requested filler and a corrected free run completes", async ({ page, host }) => {
    await host.open(page);
    await page.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
    await page.getByLabel("Label *", { exact: true }).fill("Optional setup experiment");
    await page.getByLabel("Use optional filler", { exact: true }).selectOption("true");
    await page.getByRole("button", { name: "Run", exact: true }).click();
    await expect(page.getByText("No run created.", { exact: true })).toBeVisible();
    await expect(page.getByRole("region", { name: "Result content" }).getByText(/Setup needed: optional filler/)).toBeVisible();
    await page.getByLabel("Use optional filler", { exact: true }).selectOption("false");
    await expect(page.getByRole("button", { name: "Run", exact: true })).toBeEnabled();
    await page.getByRole("button", { name: "Run", exact: true }).click();
    await expect(page.getByText("Contract evidence: complete.", { exact: false }).first()).toBeVisible();
    await expect(page.getByRole("slider", { name: "Before and after divider" })).toBeVisible();
  });
});
