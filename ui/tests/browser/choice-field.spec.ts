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
    // The default first; the one not installed is marked and cannot be picked yet.
    await expect(model.locator("option").nth(1)).toHaveText("small");
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
    expect(dispatched).toBe(0);
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
    await page.keyboard.press("Escape");
    await expect(page.getByRole("status").filter({ hasText: "small is not installed" })).toContainText("Get the small model in the tool's setup.");
    await page.getByRole("button", { name: "Open tool setup" }).click();
    await expect(page.getByRole("dialog", { name: "Tool setup", exact: true }).getByRole("region", { name: "Installables" })).toBeVisible();
  });
});
