import { prepareSampleInput } from "./sample-input";
import { test, expect } from "./tool-host";
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
test.use({ hostScenario: "held" });
test("another tab follows a run after a stream reconnect and the launching tab closes", async ({
  page,
  context,
  host,
}) => {
  test.setTimeout(30000);
  await host.open(page);
  await prepareSampleInput(page, "Survives a closed tab");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  // The other tab follows the run at its address (route.ts), through a dropped stream.
  await expect(page).toHaveURL(/\/sample\/mark_band\/[0-9a-f-]{36}$/);
  const address = new URL(page.url()).pathname;
  const other = await context.newPage();
  let connections = 0;
  await other.route("**/api/events", async (route) => {
    connections++;
    if (connections === 1) await route.abort();
    else await route.continue();
  });
  await host.open(other, address);
  await expect.poll(() => connections).toBeGreaterThanOrEqual(2);
  await expect(
    other.getByRole("status").filter({ hasText: "running" }),
  ).toBeVisible();
  await expect(
    other.getByText("Waiting in the offline test tool", { exact: true }),
  ).toBeVisible();
  await page.close();
  // Complete only after the launching tab is gone; scheduler timing cannot release it early.
  await writeFile(resolve(host.root, "../tool/release-run"), "");
  await expect(
    other.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible({ timeout: 15000 });
  await expect(other.getByRole("slider")).toBeVisible();
  const calls = await readFile(
    resolve(host.root, "../tool/calls.ndjson"),
    "utf8",
  );
  expect(calls.trim().split("\n")).toHaveLength(1);
  await other.close();
});
