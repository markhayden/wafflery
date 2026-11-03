import { expect, type Page } from "@playwright/test";

/** Establish a usable draft before a test exercises runs, history or layout. */
export async function prepareSampleInput(page: Page, text: string) {
  const source = page.getByRole("region", { name: "source", exact: true });
  await source.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
  await expect(source).toHaveAttribute("aria-busy", "false");
  await expect(source.getByRole("img", { name: "Selected image 1" })).toBeVisible();
  await editSampleLabel(page, text);
}

/** Set fixture input in one DOM task, then verify React committed it, not just the DOM value.
 * Native keyboard behavior is covered separately in draft-edit.spec.ts.
 */
export async function editSampleLabel(page: Page, text: string) {
  const label = page.getByLabel("Label", { exact: true });
  await expect(label).toBeEnabled();
  await label.evaluate((node: HTMLInputElement, value) => {
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
    if (!setter) throw new Error("Input value setter is unavailable");
    // Bypass React's instance tracker, then deliver the same bubbling input event it observes.
    setter.call(node, value);
    node.dispatchEvent(new InputEvent("input", {
      bubbles: true, composed: true, inputType: "insertReplacementText", data: value,
    }));
  }, text);
  await expect(label).toHaveValue(text);
  await expect(label).toHaveAttribute("value", text);
}
