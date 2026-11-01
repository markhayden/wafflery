import { test, expect } from "./tool-host";

test("setup shows reported allowances and never promotes a credential name to readiness", async ({ page }) => {
  await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-toolsetup--capped&viewMode=story");
  await expect(page.getByRole("region", { name: "Session spending" })).toContainText("$1.80");
  await expect(page.getByText(/final charge is unknown/)).toContainText("$0.04");
  await expect(page.getByText("EXAMPLE_API_KEY", { exact: true })).toBeVisible();
  await expect(page.getByText(/availability and validity have not been checked/)).toBeVisible();
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
  await page.getByLabel("Label *", { exact: true }).fill("Keep this draft");
  let dispatched = 0;
  page.on("request", request => { if (request.method() === "POST" && new URL(request.url()).pathname === "/api/attempts") dispatched++; });
  await page.getByRole("button", { name: "Tool setup", exact: true }).click();
  const drawer = page.getByRole("dialog", { name: "Tool setup", exact: true });
  await expect(drawer.getByRole("region", { name: "Session spending" })).toContainText("Uncapped");
  await expect(drawer).toContainText("No credential variables are declared");
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
