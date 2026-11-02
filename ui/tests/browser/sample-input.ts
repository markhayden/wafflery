import { expect, type Page } from "@playwright/test";

/** Establish a usable draft before a test exercises runs, history or layout. */
export async function prepareSampleInput(page: Page, text: string) {
  const source = page.getByRole("region", { name: "source", exact: true });
  await source.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
  await expect(source).toHaveAttribute("aria-busy", "false");
  await expect(source.getByRole("img", { name: "Selected image 1" })).toBeVisible();
  const label = page.getByLabel("Label *", { exact: true });
  await label.click();
  await expect(label).toBeFocused();
  await label.fill(text);
  await expect(label).toHaveValue(text);
}
