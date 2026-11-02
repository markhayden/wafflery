import { test, expect } from "./tool-host";
import { savedRun, frame } from "./report-fixture";
import { createServer } from "node:http";
import { writeFile } from "node:fs/promises";
import { join } from "node:path";

test.use({ hostScenario: "renders" });

test("untrusted reports cannot read parent state, navigate the workspace or open popups", async ({ page, host, context }) => {
  const { api, id } = await savedRun(page, host);
  const workspace = page.url();
  for (const name of ["server-probe", "document-probe"]) {
    const view = await api(`/api/attempts/${id}/views`, "POST", { name });
    await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("ready");
    const grant = await api(`/api/views/${view.id}/frame`, "POST", {});
    const report = await frame(page, host, grant.url);
    await report.getByRole("button", { name: "Probe parent" }).click();
    await expect(report.locator("output")).toHaveText("parent:blocked;storage:blocked;top:blocked;popup:blocked");
    expect(page.url()).toBe(workspace);
    expect(context.pages()).toHaveLength(1);
    await page.locator('iframe[title="Report wrapper"]').evaluate((node) => node.remove());
    await api(`/api/views/${view.id}/close`, "POST", {});
    await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("exited");
  }
});

test("a server report cannot authenticate workspace API calls or submit forms", async ({ page, host }) => {
  const { api, id } = await savedRun(page, host);
  const before = await api(`/api/attempts/${id}`);
  const view = await api(`/api/attempts/${id}/views`, "POST", { name: "server-probe" });
  await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("ready");
  const grant = await api(`/api/views/${view.id}/frame`, "POST", {});
  const report = await frame(page, host, grant.url);
  const attempted = page.waitForRequest((request) => request.url() === `${host.origin}/api/host/quit`
    && request.method() === "POST");
  await report.getByRole("button", { name: "Probe API" }).click();
  expect((await attempted).headers()["authorization"]).toBeUndefined();
  await expect(report.locator("output")).toHaveText("api:blocked");
  // CORS failures intentionally hide the response from browser instrumentation.
  // Confirm the same unauthenticated foreign-origin request is rejected by the host.
  const source = await page.frameLocator('iframe[title="Report wrapper"]')
    .locator('iframe[title="Tool report"]').getAttribute("src");
  const denied = await page.request.post(`${host.origin}/api/host/quit`, {
    headers: { origin: new URL(source!).origin, "content-type": "text/plain" }, data: "{}",
  });
  expect(denied.status()).toBe(403);
  const formBlocked = page.waitForEvent("console", (message) => /form/i.test(message.text()) && /sandbox/i.test(message.text()));
  await report.getByRole("button", { name: "Submit form" }).click();
  await formBlocked;
  await expect(report.getByRole("heading", { name: "Isolation probe" })).toBeVisible();
  expect(await api(`/api/attempts/${id}`)).toEqual(before);
});

test("wrapper policy blocks report navigation and redirects to another loopback origin", async ({ page, host }) => {
  let arrivals = 0;
  const canary = createServer((_request, response) => { arrivals++; response.end("unapproved origin"); });
  await new Promise<void>((resolve) => canary.listen(0, "127.0.0.1", resolve));
  try {
    const address = canary.address();
    if (!address || typeof address === "string") throw new Error("No canary listener");
    const target = `http://127.0.0.1:${address.port}/unapproved`;
    await writeFile(join(host.root, "..", "tool", "report-target"), target);
    const { api, id } = await savedRun(page, host);
    for (const link of ["Other origin", "Redirect"]) {
      const view = await api(`/api/attempts/${id}/views`, "POST", { name: "server-probe" });
      await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("ready");
      const grant = await api(`/api/views/${view.id}/frame`, "POST", {});
      const report = await frame(page, host, grant.url);
      await expect(report.getByRole("heading", { name: "Isolation probe" })).toBeVisible();
      const wrapper = page.frameLocator('iframe[title="Report wrapper"]');
      await wrapper.locator("body").evaluate((body) => {
        document.addEventListener("securitypolicyviolation", (event) => {
          body.dataset.violation = event.effectiveDirective;
        });
      });
      await report.getByRole("link", { name: link, exact: true }).click();
      await expect(wrapper.locator("body")).toHaveAttribute("data-violation", "frame-src");
      expect(arrivals).toBe(0);
      await page.locator('iframe[title="Report wrapper"]').evaluate((node) => node.remove());
      await api(`/api/views/${view.id}/close`, "POST", {});
      await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("exited");
    }
  } finally {
    await new Promise<void>((resolve, reject) => canary.close((error) => error ? reject(error) : resolve()));
  }
});
