import { openOptionalSettings, prepareSampleInput } from "./sample-input";
import { test, expect } from "./tool-host";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
test("an example owns saved inputs and opens a draft without execution", async ({
  page,
  host,
}) => {
  await host.open(page);
  await prepareSampleInput(page, "Saved example input");
  await openOptionalSettings(page);
  await page.getByLabel("Tags", { exact: true }).fill('["saved tag"]');
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(
    page.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible();
  await page.getByLabel("Label", { exact: true }).fill("Unsubmitted edits");
  await page.getByLabel("Tags", { exact: true }).fill("invalid draft JSON");
  await page.getByRole("button", { name: "Examples", exact: true }).click();
  const drawer = page.getByRole("dialog", { name: "Examples", exact: true });
  await drawer
    .getByLabel("Example name", { exact: true })
    .fill("My first example");
  await drawer
    .getByLabel("Notes", { exact: true })
    .fill("Try this again later");
  await drawer
    .getByRole("button", { name: "Save example", exact: true })
    .click();
  await drawer.getByRole("button", { name: "Open My first example" }).click();
  await expect(page.getByLabel("Label", { exact: true })).toHaveValue(
    "Saved example input",
  );
  await expect(page.getByLabel("Tags", { exact: true })).toHaveValue(
    /saved tag/,
  );
  await expect(
    page.getByRole("status").filter({ hasText: "Ready" }),
  ).toBeVisible();
  const result = page.getByRole("region", { name: "Result", exact: true });
  await expect(result).toContainText("Example loaded: My first example");
  await expect(result).toContainText("Click Run to create a new result.");
  expect(
    (await readFile(resolve(host.root, "../tool/calls.ndjson"), "utf8"))
      .trim()
      .split("\n"),
  ).toHaveLength(1);
  await result.getByRole("button", { name: "Browse run history" }).click();
  await page.getByRole("dialog", { name: "History", exact: true })
    .getByRole("button", { name: /^Add a test band$/ }).click();
  await expect(page.getByText("From example:", { exact: false })).toHaveCount(0);
  await page.getByRole("button", { name: "Examples", exact: true }).click();
  await drawer.getByRole("button", { name: "Open My first example" }).click();
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(
    page.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible();
  expect(
    (await readFile(resolve(host.root, "../tool/calls.ndjson"), "utf8"))
      .trim()
      .split("\n"),
  ).toHaveLength(2);
});
