import { test, expect } from "./tool-host";
import { prepareSampleInput } from "./sample-input";

test.use({ hostScenario: "estimates" });

test("native keyboard edits after an uncertain paid reply preserve the original submission", async ({ page, host }) => {
  await host.open(page);
  await page.getByRole("button", { name: /^Action / }).click();
  await page.getByRole("button", { name: /^Price a scripted call/ }).click();
  await prepareSampleInput(page, "reviewed paid draft");
  await page.getByRole("textbox", { name: "Spending cap (USD)", exact: true }).fill("0.5");
  await page.getByRole("button", { name: "Estimate cost", exact: true }).click();
  await expect(page.getByRole("region", { name: "Cost", exact: true })).toContainText("Estimate within cap.");
  const submissions: { id: string; params: { label: string } }[] = [];
  await page.route("**/api/preparations/*/submit", async (route) => {
    submissions.push(route.request().postDataJSON());
    const response = await route.fetch();
    if (submissions.length === 1) {
      const value = await response.json();
      value.request.params.label = "wrong response";
      await route.fulfill({ response, json: value });
    } else await route.fulfill({ response });
  });
  try {
    await page.getByRole("button", { name: "Run", exact: true }).click();
    const retry = page.getByRole("button", { name: "Retry submission", exact: true });
    await expect(retry).toBeVisible();
    const label = page.getByLabel("Label", { exact: true });
    await label.click();
    await label.press("ControlOrMeta+A");
    await label.pressSequentially("keyboard after sending");
    await expect(label).toHaveValue("keyboard after sending");
    await expect(label).toHaveAttribute("value", "keyboard after sending");
    await retry.click();
    await expect(page.getByRole("region", { name: "Result", exact: true })).toContainText("Contract evidence: complete");
    expect(submissions).toHaveLength(2);
    expect(submissions[1]).toEqual(submissions[0]);
    expect(submissions[0]!.params.label).toBe("reviewed paid draft");
    await expect(label).toHaveAttribute("value", "keyboard after sending");
  } finally { await page.unrouteAll({ behavior: "wait" }); }
});
