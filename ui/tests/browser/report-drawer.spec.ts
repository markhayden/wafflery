import { test, expect } from "./tool-host";
import { savedRun, chooseReport } from "./report-fixture";

test.use({ hostScenario: "renders" });


for (const [name, kind, heading] of [["document", "Document", "Saved document"], ["server", "Live viewer", "Saved report"]] as const) {
test(`the shared drawer opens a saved ${name} explicitly and closes it when returning to evidence`, async ({ page, host }) => {
  const { api, id } = await savedRun(page, host);
  await page.getByRole("button", { name: "Inspect run", exact: true }).click();
  await page.getByRole("button",{name:"Advanced",exact:true}).click();
  await page.getByRole("button", { name: "Reports", exact: true }).click();
  expect(await api(`/api/attempts/${id}/views`)).toEqual({ view: null });
  await chooseReport(page, name, kind);
  await page.getByRole("button", { name: "Open report", exact: true }).click();
  const report = page.frameLocator('iframe[title="Report viewer"]').frameLocator('iframe[title="Tool report"]');
  await expect(report.getByRole("heading", { name: heading })).toBeVisible();
  const { view } = await api(`/api/attempts/${id}/views`);
  await page.getByRole("button", { name: "Inputs", exact: true }).click();
  await expect(page.getByLabel("Saved evidence")).toContainText("report source");
  await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("exited");
  expect((await api(`/api/attempts/${id}`)).execution).toBe("completed");
});
}

test("a failed report keeps diagnostics and saved evidence available without changing the run", async ({ page, host }) => {
  const { api, id } = await savedRun(page, host);
  await page.getByRole("button", { name: "Inspect run", exact: true }).click();
  await page.getByRole("button",{name:"Advanced",exact:true}).click();
  await page.getByRole("button", { name: "Reports", exact: true }).click();
  await chooseReport(page, "early-error", "Document");
  await page.getByRole("button", { name: "Open report", exact: true }).click();
  await expect(page.getByRole("region", { name: "Tool report", exact: true })).toContainText("Report unavailable");
  await page.getByText("Report diagnostics", { exact: true }).click();
  await expect(page.getByLabel("Report diagnostics output")).toContainText("Synthetic render failure");
  await page.getByRole("button", { name: "Result", exact: true }).click();
  await expect(page.getByLabel("Saved evidence")).toBeVisible();
  expect((await api(`/api/attempts/${id}`)).execution).toBe("completed");
});
