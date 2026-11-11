import { test, expect } from "./tool-host";
import { writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";

test.describe("a choice with installable models", () => {
  test.use({ hostScenario: "installables" });

  test("a model not installed shows so in the form, and Get in setup makes it selectable without a run", async ({ page, host }) => {
    await host.open(page);
    let dispatched = 0;
    page.on("request", request => { if (request.method() === "POST" && new URL(request.url()).pathname === "/api/attempts") dispatched++; });
    const model = page.getByLabel("Model", { exact: true });
    // The choice the readiness depends on is in the main settings, right after the required
    // label, with the default selected rather than "Not set".
    await expect(model).toHaveValue(JSON.stringify("small"));
    await expect(model.locator("option", { hasText: "Not set" })).toHaveCount(0);
    const settings = page.getByRole("region", { name: "Config settings", exact: true });
    const order = await settings.locator("[aria-labelledby]").evaluateAll(nodes =>
      nodes.map(node => document.getElementById(node.getAttribute("aria-labelledby") ?? "")?.textContent ?? ""));
    expect(order.slice(order.indexOf("Label"), order.indexOf("Label") + 2)).toEqual(["Label", "Model"]);
    // The default first; the one not installed is marked and cannot be picked yet.
    await expect(model.locator("option").nth(0)).toHaveText("small");
    await expect(model.locator("option", { hasText: "large" })).toHaveText("large · not installed");
    await expect(model.locator("option", { hasText: "large" })).toHaveAttribute("disabled", "");

    await page.getByRole("button", { name: "Tool setup", exact: true }).click();
    const drawer = page.getByRole("dialog", { name: "Tool setup", exact: true });
    const models = drawer.getByRole("region", { name: "Installables" });
    const large = models.getByRole("listitem", { name: "Large model" });
    await expect(large).toContainText("Best for whole objects · lab remove score 0.777");
    await expect(large).toContainText("3.4 GB download · Example Model License");
    await large.getByRole("button", { name: "Get", exact: true }).click();
    await expect(large.getByRole("group", { name: "Confirm getting Large model" })).toContainText("Download 3.4 GB");
    await large.getByRole("button", { name: "Get Large model" }).click();
    await expect(large).toContainText("Installing");
    await expect(large.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "25");

    for (const width of [320, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      await expect(large).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    }
    await page.screenshot({ path: "test-results/installables-installing.png" });

    await writeFile(join(dirname(host.root), "tool", "release-install"), "");
    await expect(large).toContainText("Ready");
    await expect(large.getByRole("button", { name: "Remove" })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(model.locator("option", { hasText: "large" })).toHaveText("large");
    await expect(model.locator("option", { hasText: "large" })).not.toHaveAttribute("disabled");
    // Picking it brings its requirement into setup, in place of the default's.
    await model.selectOption({ label: "large" });
    await page.getByRole("button", { name: "Tool setup", exact: true }).click();
    const setup = page.getByRole("dialog", { name: "Tool setup", exact: true }).getByRole("region", { name: "Provider setup" });
    await setup.getByText(/^Show requirements/).click();
    await expect(setup.getByRole("list", { name: "Other requirements" })).toContainText("model:large");
    await expect(setup.getByRole("list", { name: "Other requirements" })).not.toContainText("model:small");
    await page.keyboard.press("Escape");
    expect(dispatched).toBe(0);
  });

  test("a ready action shows Ready, its requirements collapsed and only the chosen model's", async ({ page, host }) => {
    await host.open(page);
    await page.getByRole("button", { name: "Tool setup", exact: true }).click();
    const setup = page.getByRole("dialog", { name: "Tool setup", exact: true }).getByRole("region", { name: "Provider setup" });
    await expect(setup.getByText("Ready", { exact: true })).toBeVisible();
    // The optional key is missing, yet nothing is listed as needed.
    await expect(setup.getByRole("list", { name: "Missing requirements" })).toHaveCount(0);
    const others = setup.getByRole("list", { name: "Other requirements" });
    await expect(others).not.toBeVisible();
    await setup.getByText("Show requirements (4)", { exact: true }).click();
    for (const name of ["sample-lab.toml", "SAMPLE_MODELS", "model:small", "SAMPLE_CHECK_KEY"])
      await expect(others).toContainText(name);
    await expect(others).not.toContainText("model:large");
    await expect(others.getByRole("listitem").filter({ hasText: "SAMPLE_CHECK_KEY" })).toContainText("Missing · optional");
    await expect(others.getByRole("listitem").filter({ hasText: "model:small" })).toContainText("Needed because model is small.");
    // "Not validated" is said once for the whole list.
    await expect(setup.getByText(/has not checked that it works/)).toHaveCount(1);
    await expect(setup).not.toContainText("not validated");
  });

  test("a missing default says so in the form and opens setup", async ({ page, host }) => {
    await writeFile(join(dirname(host.root), "tool", "installables", "small"), "not_installed").catch(async () => {
      const { mkdir } = await import("node:fs/promises");
      await mkdir(join(dirname(host.root), "tool", "installables"), { recursive: true });
      await writeFile(join(dirname(host.root), "tool", "installables", "small"), "not_installed");
    });
    await host.open(page);
    // The host read the description at start; Refresh rereads it with the default now missing.
    await page.getByRole("button", { name: "Tool setup", exact: true }).click();
    await page.getByRole("dialog", { name: "Tool setup", exact: true }).getByRole("button", { name: "Refresh", exact: true }).click();
    await expect(page.getByRole("dialog", { name: "Tool setup", exact: true }).getByRole("listitem", { name: "Small model" })).toContainText("Not installed");
    // The missing default is the one thing listed open; the rest stay collapsed.
    const missing = page.getByRole("dialog", { name: "Tool setup", exact: true }).getByRole("list", { name: "Missing requirements" });
    await expect(missing).toContainText("model:small");
    await expect(missing).not.toContainText("sample-lab.toml");
    await expect(page.getByRole("dialog", { name: "Tool setup", exact: true }).getByText("Setup needed", { exact: true })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("status").filter({ hasText: "small is not installed" })).toContainText("Get the small model in the tool's setup.");
    await page.getByRole("button", { name: "Open tool setup" }).click();
    await expect(page.getByRole("dialog", { name: "Tool setup", exact: true }).getByRole("region", { name: "Installables" })).toBeVisible();
  });
});
