import { expect, type Host } from "./tool-host";
import { prepareSampleInput, editSampleLabel } from "./sample-input";
import type { Page } from "@playwright/test";

export async function savedRun(page: Page, host: Host) {
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

export async function frame(page: Page, host: Host, url: string) {
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


export async function chooseReport(page: Page, name: string, kind: string) {
  await page.getByRole("button", { name: /^Report / }).click();
  await page.getByRole("dialog", { name: "Choose report", exact: true })
    .getByRole("button", { name: `${name} ${kind}`, exact: true }).click();
}
