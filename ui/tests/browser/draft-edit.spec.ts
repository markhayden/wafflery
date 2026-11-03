import { test, expect } from "./tool-host";
import { editSampleLabel, prepareSampleInput } from "./sample-input";

test("fixture edits commit an input event even if focus changes before native insertion", async ({ page, host }) => {
  await host.open(page);
  await prepareSampleInput(page, "original");
  const label = page.getByLabel("Label", { exact: true });
  // Reproduce the gap between Playwright's DOM selection and its separate insertText call.
  await label.evaluate((node: HTMLInputElement) => {
    node.addEventListener("select", () => node.blur());
  });
  await editSampleLabel(page, "committed draft");
  await expect(label).toHaveAttribute("value", "committed draft");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.getByRole("region", { name: "Result", exact: true })).toContainText("Contract evidence: complete");
  await page.getByRole("button", { name: "Inspect run", exact: true }).click();
  await expect(page.getByLabel("Saved evidence")).toContainText('"label": "committed draft"');
});

test("native keyboard editing reaches the saved run through the controlled form", async ({ page, host }) => {
  await host.open(page);
  await prepareSampleInput(page, "original");
  const label = page.getByLabel("Label", { exact: true });
  await label.click();
  await label.press("ControlOrMeta+A");
  await label.pressSequentially("keyboard draft");
  await expect(label).toHaveAttribute("value", "keyboard draft");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.getByRole("region", { name: "Result", exact: true })).toContainText("Contract evidence: complete");
  await page.getByRole("button", { name: "Inspect run", exact: true }).click();
  await expect(page.getByLabel("Saved evidence")).toContainText('"label": "keyboard draft"');
});
