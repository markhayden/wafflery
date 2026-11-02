import { prepareSampleInput } from "./sample-input";
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
  await expect(page.getByText(/Paid execution needs/)).toContainText("Review the estimate");
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
  const before = await drawer.getByRole("region", { name: "Session spending" }).locator("code").innerText();
  await drawer.getByRole("button", { name: "Refresh", exact: true }).click();
  await expect(drawer.getByRole("region", { name: "Session spending" })).toContainText("Uncapped");
  await expect(drawer.getByRole("region", { name: "Session spending" }).locator("code")).toHaveText(before);
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
    await expect(page.getByRole("dialog")).toContainText("Review the estimate");
    await page.keyboard.press("Escape");
    await expect(page.getByRole("button", { name: "Run", exact: true })).toBeDisabled();
  });
});

test("an unavailable tool never replaces setup with a saved-run inspector", async ({ page, host }) => {
  await host.open(page);
  await page.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
  const label = page.getByLabel("Label *", { exact: true });
  await label.click();
  await expect(label).toBeFocused();
  await label.fill("Saved before setup");
  await expect(label).toHaveValue("Saved before setup");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.getByText("Contract evidence: complete.", { exact: false }).first()).toBeVisible();
  await page.getByRole("button", { name: "Tool setup", exact: true }).click();
  const drawer = page.getByRole("dialog", { name: "Tool setup", exact: true });
  await expect(drawer).toContainText("Uncapped");
  let armed = false;
  await page.route("**/api/tools/sample/refresh", async route => {
    armed = true;
    await route.continue();
  });
  await page.route("**/api/state", async route => {
    const makeUnavailable = armed;
    const response = await route.fetch();
    const value = await response.json();
    if (makeUnavailable) value.tools = value.tools.map((tool: {id: string}) => ({ ...tool, declaration: null, error: "Tool description is unavailable" }));
    await route.fulfill({ response, json: value });
  });
  try {
    const changed = page.waitForResponse(async response => {
      if (new URL(response.url()).pathname !== "/api/state" || response.status() !== 200) return false;
      const value = await response.json();
      return value.tools.some((tool: { declaration: unknown }) => tool.declaration === null);
    });
    await Promise.all([
      changed,
      drawer.getByRole("button", { name: "Refresh", exact: true }).click(),
    ]);
    await expect(drawer).toContainText("Setup is unavailable because this tool or action is no longer available.");
    await expect(drawer.getByRole("navigation", { name: "Run evidence" })).not.toBeVisible();
    await expect(drawer).not.toContainText("Uncapped");
  } finally {
    // Also drain interception when an assertion fails, before the host fixture stops.
    await page.unrouteAll({ behavior: "wait" });
  }
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
    await prepareSampleInput(page, "Optional setup experiment");
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

test("restart confirmation preserves drafts and publishes a fresh session to both tabs", async ({ page, context, host }, testInfo) => {
  await host.open(page);
  const label = page.getByLabel("Label *", { exact: true });
  await label.click(); await label.fill("Keep across restart");
  await expect(label).toHaveValue("Keep across restart");
  const other = await context.newPage(); await host.open(other);
  for (const tab of [page, other]) await tab.getByRole("button", { name: "Tool setup", exact: true }).click();
  const drawer = page.getByRole("dialog", { name: "Tool setup", exact: true });
  const otherDrawer = other.getByRole("dialog", { name: "Tool setup", exact: true });
  const spending = drawer.getByRole("region", { name: "Session spending" });
  await expect(spending).toContainText("Uncapped");
  await spending.getByText("Session identity", { exact: true }).click();
  const before = await spending.locator("code").innerText();
  let restarts = 0, attempts = 0;
  page.on("request", request => {
    if (request.method() !== "POST") return;
    if (new URL(request.url()).pathname.endsWith("/restart")) restarts++;
    if (new URL(request.url()).pathname === "/api/attempts") attempts++;
  });
  await drawer.getByRole("button", { name: "Restart tool session", exact: true }).click();
  await expect(drawer).toContainText("new spending ledger");
  await expect(drawer.getByRole("button", { name: "Keep current session", exact: true })).toBeFocused();
  await page.screenshot({ path: testInfo.outputPath("restart-confirmation.png") });
  await drawer.getByRole("button", { name: "Keep current session", exact: true }).click();
  expect(restarts).toBe(0);
  await expect(drawer.getByRole("button", { name: "Restart tool session", exact: true })).toBeFocused();
  await expect(spending.locator("code")).toHaveText(before);
  await drawer.getByRole("button", { name: "Restart tool session", exact: true }).click();
  await drawer.getByRole("button", { name: "Confirm restart", exact: true }).click();
  const restarted = drawer.getByRole("region", { name: "Restart tool" });
  await expect(restarted).toContainText("Tool restarted");
  const after = await restarted.locator("code").innerText();
  expect(after).not.toBe(before);
  const otherSpending = otherDrawer.getByRole("region", { name: "Session spending" });
  await expect(otherSpending).toContainText("Uncapped");
  await otherSpending.getByText("Session identity", { exact: true }).click();
  await expect(otherSpending.locator("code")).toHaveText(after);
  await page.keyboard.press("Escape");
  await expect(label).toHaveValue("Keep across restart");
  expect(restarts).toBe(1); expect(attempts).toBe(0);
  await other.close();
});

test.describe("restart during work", () => {
  test.use({ hostScenario: "held" });
  test("setup explains why an active session cannot restart", async ({ page, host }) => {
    await host.open(page);
    await page.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
    const source = page.getByRole("region", { name: "source", exact: true });
    await expect(source).toHaveAttribute("aria-busy", "false");
    await expect(source.getByRole("img", { name: "Selected image 1" })).toBeVisible();
    const label = page.getByLabel("Label *", { exact: true });
    await label.click();
    await expect(label).toBeFocused();
    await label.fill("Held run");
    await expect(label).toHaveValue("Held run");
    await page.getByRole("button", { name: "Run", exact: true }).click();
    await expect(page.getByRole("button", { name: "Cancel run", exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Tool setup", exact: true }).click();
    const drawer = page.getByRole("dialog", { name: "Tool setup", exact: true });
    await expect(drawer.getByRole("button", { name: "Restart tool session", exact: true })).toBeDisabled();
    await expect(drawer).toContainText("active or unconfirmed work");
    await writeFile(join(host.root, "..", "tool", "release-run"), "");
    await expect(drawer.getByRole("button", { name: "Restart tool session", exact: true })).toBeEnabled();
  });
});

test("failed restart keeps deliberate recovery reachable in setup", async ({ page, host }) => {
  await host.open(page);
  await page.getByRole("button", { name: "Tool setup", exact: true }).click();
  const drawer = page.getByRole("dialog", { name: "Tool setup", exact: true });
  await page.route("**/api/tools/sample/restart", route => route.fulfill({ status: 422, contentType: "application/json", body: JSON.stringify({ error: { message: "Replacement initialization failed" } }) }));
  await page.route("**/api/state", async route => {
    const response = await route.fetch(); const value = await response.json();
    value.tools = value.tools.map((tool: {id: string}) => ({ ...tool, declaration: null, error: "Disconnected after failed restart" }));
    await route.fulfill({ response, json: value });
  });
  await drawer.getByRole("button", { name: "Restart tool session", exact: true }).click();
  await drawer.getByRole("button", { name: "Confirm restart", exact: true }).click();
  await expect(drawer).toContainText("Replacement initialization failed");
  await expect(drawer).not.toContainText("Uncapped");
  await page.unrouteAll({ behavior: "wait" });
  await drawer.getByRole("button", { name: "Restart tool session", exact: true }).click();
  await drawer.getByRole("button", { name: "Confirm restart", exact: true }).click();
  await expect(drawer.getByRole("region", { name: "Restart tool" })).toContainText("Tool restarted");
});


test("restart stories fit small screens and explain busy or disconnected states", async ({ page }, testInfo) => {
  await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-restarttool--idle&viewMode=story");
  await page.getByRole("button", { name: "Restart tool session", exact: true }).click();
  for (const width of [320, 768, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    await expect(page.getByRole("button", { name: "Confirm restart", exact: true })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    if (width === 320) await page.screenshot({ path: testInfo.outputPath("restart-small-screen.png") });
  }
  await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-restarttool--busy&viewMode=story");
  await expect(page.getByRole("button", { name: "Restart tool session", exact: true })).toBeDisabled();
  await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-restarttool--disconnected&viewMode=story");
  await page.getByRole("button", { name: "Restart tool session", exact: true }).click();
  await page.getByRole("button", { name: "Confirm restart", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("could not initialize");
  await expect(page.getByRole("button", { name: "Restart tool session", exact: true })).toBeEnabled();
});

test("live folder setup reports launch permissions without inventing effective access", async ({page,host}, testInfo) => {
  await host.open(page);
  await page.getByRole("button", {name:"Tool setup",exact:true}).click();
  const drawer = page.getByRole("dialog",{name:"Tool setup",exact:true});
  const access = drawer.getByRole("region",{name:"Folder access"});
  const effective = access.getByRole("group",{name:"Effective input folders"});
  const requested = access.getByRole("group",{name:"Requested input folders"});
  const results = access.getByRole("group",{name:"Result folders"});
  await expect(effective).toContainText("Not reported");
  await expect(effective.locator("code")).toHaveCount(0);
  await expect(requested.locator("code")).toHaveCount(1);
  await expect(requested.locator("code")).toContainText("/data/Library/execution");
  await expect(results.locator("code")).toHaveCount(1);
  await expect(results.locator("code")).toContainText("/tool");
  const before = await requested.locator("code").allTextContents();
  await page.route("**/api/tools/sample/session", route => route.fulfill({status:503,contentType:"application/json",body:JSON.stringify({error:{message:"Session temporarily unavailable"}})}));
  try {
    await drawer.getByRole("button",{name:"Refresh",exact:true}).click();
    await expect(effective).toContainText("Unavailable");
    await expect(effective).not.toContainText("Not reported");
    await expect(effective).not.toContainText("No file paths allowed.");
    await expect(requested.locator("code")).toHaveText(before);
  } finally {
    await page.unrouteAll({behavior:"wait"});
  }
  await drawer.getByRole("button",{name:"Refresh",exact:true}).click();
  await expect(effective).toContainText("Not reported");
  await access.scrollIntoViewIfNeeded();
  await access.screenshot({path:testInfo.outputPath("live-folder-access.png")});
});
