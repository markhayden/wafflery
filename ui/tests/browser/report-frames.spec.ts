import { test, expect, type Host } from "./tool-host";
import { prepareSampleInput, editSampleLabel } from "./sample-input";
import type { Page } from "@playwright/test";

test.use({ hostScenario: "renders" });

async function savedRun(page: Page, host: Host) {
  const session = page.waitForResponse(
    (response) => new URL(response.url()).pathname === "/api/session",
  );
  await host.open(page);
  const { token } = await (await session).json();
  const api = async (path: string, method = "GET", data?: unknown) => {
    const response = await page.request.fetch(`${host.origin}${path}`, {
      method, data,
      headers: { authorization: `Bearer ${token}`, origin: host.origin },
    });
    expect(response.ok()).toBe(true);
    return response.json();
  };
  await prepareSampleInput(page, "report source");
  const submitted = page.waitForResponse(
    (response) => new URL(response.url()).pathname === "/api/attempts"
      && response.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Run", exact: true }).click();
  const { id } = await (await submitted).json();
  await expect.poll(async () =>
    (await api(`/api/attempts/${id}`)).preservation.contract,
  ).toBe("complete");
  await editSampleLabel(page, "later draft");
  return { api, id };
}

async function frame(page: Page, host: Host, url: string) {
  await page.evaluate((url) => {
    const iframe = document.createElement("iframe");
    iframe.title = "Report wrapper";
    iframe.src = url;
    iframe.style.cssText = "position:fixed;inset:5%;width:90%;height:90%;z-index:999;background:white";
    document.body.append(iframe);
  }, `${host.origin}${url}`);
  return page.frameLocator('iframe[title="Report wrapper"]')
    .frameLocator('iframe[title="Tool report"]');
}

test("server and document reports load inside the exact-origin trusted wrapper", async ({ page, host }) => {
  const { api, id } = await savedRun(page, host);
  const requests: { url: string; authorization: string | undefined; referer: string | undefined }[] = [];
  page.on("request", (request) => requests.push({
    url: request.url(),
    authorization: request.headers()["authorization"],
    referer: request.headers()["referer"],
  }));
  for (const name of ["server", "document"]) {
    const view = await api(`/api/attempts/${id}/views`, "POST", { name });
    await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("ready");
    const grant = await api(`/api/views/${view.id}/frame`, "POST", {});
    const report = await frame(page, host, grant.url);
    await expect(report.getByRole("heading", {
      name: name === "server" ? "Saved report" : "Saved document",
    })).toBeVisible();
    const wrapper = page.frameLocator('iframe[title="Report wrapper"]');
    const source = await wrapper.locator('iframe[title="Tool report"]').getAttribute("src");
    expect(new URL(source!).origin).not.toBe(host.origin);
    expect(await wrapper.locator("iframe").getAttribute("sandbox"))
      .toBe("allow-scripts allow-same-origin");
    const reportRequests = requests.filter((request) => request.url === source);
    expect(reportRequests.length).toBeGreaterThan(0);
    for (const request of reportRequests) {
      expect(request.authorization).toBeUndefined();
      expect(request.referer).toBeUndefined();
    }
    const replay = await page.request.get(`${host.origin}${grant.url}`);
    expect(replay.status()).toBe(410);
    await page.locator('iframe[title="Report wrapper"]').evaluate((node) => node.remove());
    await api(`/api/views/${view.id}/close`, "POST", {});
    await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("exited");
  }
});

test("a document resource violation reaches the owner and preserves saved evidence", async ({ page, host }) => {
  const { api, id } = await savedRun(page, host);
  const before = await api(`/api/attempts/${id}`);
  const view = await api(`/api/attempts/${id}/views`, "POST", { name: "document" });
  await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("ready");
  const grant = await api(`/api/views/${view.id}/frame`, "POST", {});
  const report = await frame(page, host, grant.url);
  await expect(report.getByRole("heading", { name: "Saved document" })).toBeVisible();
  await report.locator("body").evaluate((body) => {
    const image = document.createElement("img");
    image.src = "http://127.0.0.1:9/blocked-resource";
    body.append(image);
  });
  await expect.poll(async () => (await api(`/api/views/${view.id}`)).error)
    .toBe("Report requires content blocked by its isolation policy");
  await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("exited");
  expect(await api(`/api/attempts/${id}`)).toEqual(before);
});
