import { test, expect } from "./tool-host";
import { savedRun, chooseReport } from "./report-fixture";
import { writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import type { Page } from "@playwright/test";

test.use({ hostScenario: "renders" });
const drawer = (page: Page) => page.getByRole("dialog", { name:"Run details", exact:true });
const report = (page: Page) => page.frameLocator('iframe[title="Report viewer"]').frameLocator('iframe[title="Tool report"]');
async function inspect(page: Page) {
  await page.getByRole("button", { name:"Inspect run", exact:true }).click();
  await page.getByRole("button", { name:"Reports", exact:true }).click();
}
async function open(page: Page, name: string, kind = "Document") {
  await chooseReport(page,name,kind);
  await page.getByRole("button",{name:"Open report",exact:true}).click();
}

test("dismissing during generation releases the view before inspection can reopen", async ({page,host}) => {
  const {api,id} = await savedRun(page,host);
  await inspect(page); await open(page,"document-gated");
  await expect(drawer(page)).toContainText("Starting report…");
  const {view} = await api(`/api/attempts/${id}/views`);
  expect(view.phase).toBe("starting");
  await drawer(page).getByRole("button",{name:"Close",exact:true}).click();
  await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("exited");
  await writeFile(resolve(host.root,"../tool/release-document"),"release");
  await expect(page.locator('iframe[title="Report viewer"]')).toHaveCount(0);
  await inspect(page); await open(page,"document");
  await expect(report(page).getByRole("heading",{name:"Saved document"})).toBeVisible();
  expect((await api(`/api/attempts/${id}/views`)).view.id).not.toBe(view.id);
  expect((await api(`/api/attempts/${id}`)).execution).toBe("completed");
});

test("selecting a replacement closes the old view without starting the new one", async ({page,host,browserName}) => {
  const {api,id} = await savedRun(page,host);
  await inspect(page); await open(page,"server","Live viewer");
  await expect(report(page).getByRole("heading",{name:"Saved report"})).toBeVisible();
  const {view} = await api(`/api/attempts/${id}/views`);
  await chooseReport(page,"document","Document");
  await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("exited");
  expect((await api(`/api/attempts/${id}/views`)).view.id).toBe(view.id);
  await expect(page.locator('iframe[title="Report viewer"]')).toHaveCount(0);
  await page.getByRole("button",{name:"Open report",exact:true}).click();
  await expect(report(page).getByRole("heading",{name:"Saved document"})).toBeVisible();
  await report(page).getByRole("heading").click();
  // Keyboard events inside the foreign-origin report do not reach the host dialog.
  await page.keyboard.press(browserName === "webkit" && process.platform === "darwin" ? "Alt+Shift+Tab" : "Shift+Tab");
  await expect(drawer(page).locator("button:focus")).toHaveCount(1);
  await page.keyboard.press("Escape");
  await expect(drawer(page)).not.toBeVisible();
  const current = (await api(`/api/attempts/${id}/views`)).view;
  await expect.poll(async () => (await api(`/api/views/${current.id}`)).phase).toBe("exited");
});

test("a force-required view remains owned across drawer dismissal and reopening", async ({page,host}) => {
  const {api,id} = await savedRun(page,host);
  await inspect(page); await open(page,"stuck");
  await expect(drawer(page)).toContainText("Starting report…");
  const {view} = await api(`/api/attempts/${id}/views`);
  await expect.poll(async () => (await api(`/api/views/${view.id}`)).diagnostics).toContain("synthetic renderer started");
  // The host has stopped trying gracefully, but the browser initially receives an older
  // closing observation. Remount must resume polling, rather than rely on that cached state.
  await page.route(`**/api/views/${view.id}/close`, async route => {
    const response = await route.fetch();
    await route.fulfill({response,json:{...await response.json(),phase:"stopping"}});
  });
  try {
    await drawer(page).getByRole("button",{name:"Close",exact:true}).click();
    await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("needs_force");
  } finally { await page.unrouteAll({behavior:"wait"}); }
  await inspect(page);
  await expect(page.getByRole("button",{name:"Open report",exact:true})).toBeDisabled();
  await page.getByRole("button",{name:"Force stop report",exact:true}).click();
  await expect(page.getByRole("button",{name:"Keep report",exact:true})).toBeFocused();
  expect((await api(`/api/views/${view.id}`)).phase).toBe("needs_force");
  await page.getByRole("button",{name:"Confirm force stop",exact:true}).click();
  await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("exited");
  await page.getByRole("button",{name:"Raw result",exact:true}).click();
  await expect(page.getByLabel("Saved evidence")).toBeVisible();
  expect((await api(`/api/attempts/${id}`)).execution).toBe("completed");
});

test("hidden inspection sends no activity and returning observes expiry without reopening", async ({page,host}) => {
  await page.clock.install();
  const {api,id} = await savedRun(page,host);
  await inspect(page); await open(page,"document");
  await expect(report(page).getByRole("heading",{name:"Saved document"})).toBeVisible();
  const {view} = await api(`/api/attempts/${id}/views`);
  let activity = 0, opens = 0, reads = 0;
  page.on("request",request => {
    const path = new URL(request.url()).pathname;
    if(path === `/api/views/${view.id}/activity`) activity++;
    if(path === `/api/attempts/${id}/views` && request.method()==="POST") opens++;
    if(path === `/api/views/${view.id}`) reads++;
  });
  const renewed = page.waitForResponse(response => new URL(response.url()).pathname === `/api/views/${view.id}/activity`);
  const observed = page.waitForResponse(response => new URL(response.url()).pathname === `/api/views/${view.id}`);
  await page.clock.fastForward(30_001);
  await (await renewed).finished();
  await (await observed).finished();
  await expect.poll(()=>activity).toBeGreaterThan(0);
  // Drive the visibility boundary deterministically; the Rust clock tests prove the real TTL.
  await page.evaluate(()=>{
    Object.defineProperty(document,"visibilityState",{configurable:true,get:()=>"hidden"});
    document.dispatchEvent(new Event("visibilitychange"));
  });
  const before = {activity,reads};
  await page.clock.fastForward(301_000);
  expect({activity,reads}).toEqual(before);
  await api(`/api/views/${view.id}/close`,"POST",{});
  await expect.poll(async () => (await api(`/api/views/${view.id}`)).phase).toBe("exited");
  const closed = await api(`/api/views/${view.id}`);
  // Model the expiry observation without changing the production five-minute idle duration.
  await page.route("**/api/views/**",route=> {
    const path = new URL(route.request().url()).pathname;
    return [ `/api/views/${view.id}`, `/api/views/${view.id}/activity` ].includes(path)
      ? route.fulfill({json:{...closed,idle_expired:true}}) : route.continue();
  });
  try {
    await page.evaluate(()=>{
      Object.defineProperty(document,"visibilityState",{configurable:true,get:()=>"visible"});
      document.dispatchEvent(new Event("visibilitychange"));
    });
    await expect(drawer(page)).toContainText("Report closed after inactivity.");
    await expect(page.locator('iframe[title="Report viewer"]')).toHaveCount(0);
    await expect(page.getByRole("button",{name:"Open report",exact:true})).toBeEnabled();
    expect(opens).toBe(0);
    await page.getByRole("button",{name:"Raw result",exact:true}).click();
    await expect(page.getByLabel("Saved evidence")).toBeVisible();
  } finally { await page.unrouteAll({behavior:"wait"}); }
});
