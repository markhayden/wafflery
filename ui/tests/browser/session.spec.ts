import { test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdtemp, readFile, realpath, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createConnection } from "node:net";

let child: ChildProcess;
let temporary: string;
let root: string;
let origin: string;

// Test-only private IPC client; production browser code never accesses host discovery.
async function control(operation: string): Promise<{ bootstrap: string }> {
  const canonical = await realpath(root);
  const discovery = JSON.parse(await readFile(join(root, "host.json"), "utf8")) as { socket: string };
  return new Promise((resolveReply, reject) => {
    const socket = createConnection(discovery.socket);
    let response = "";
    socket.setTimeout(2000, () => socket.destroy(new Error("control timeout")));
    socket.on("error", reject);
    socket.on("connect", () => socket.end(JSON.stringify({ protocol: 1, root: canonical, operation })));
    socket.on("data", chunk => { response += String(chunk); });
    socket.on("end", () => {
      try { resolveReply(JSON.parse(response) as { bootstrap: string }); } catch (error) { reject(error); }
    });
  });
}

async function startHost() {
  child = spawn(resolve("../target/debug/wafflehouse"), ["--foreground", "--no-open", "--data-dir", root], { stdio: ["ignore", "pipe", "pipe"] });
  origin = await new Promise<string>((resolveOrigin, reject) => {
    child.once("error", reject);
    child.once("exit", () => reject(new Error("host exited before readiness")));
    child.stdout?.once("data", chunk => resolveOrigin(String(chunk).trim()));
  });
  await expect.poll(async () => { try { await control("status"); return true; } catch { return false; } }).toBe(true);
}

async function stopHost() {
  if (child && child.exitCode === null) {
    const exited = new Promise<void>(resolveExit => child.once("exit", () => resolveExit()));
    child.kill("SIGTERM");
    await exited;
  }
}

test.beforeAll(async () => {
  temporary = await mkdtemp(join(tmpdir(), "wafflehouse-browser-"));
  root = join(temporary, "data");
  await startHost();
});

test.afterAll(async () => {
  await stopHost();
  if (temporary) await rm(temporary, { recursive: true, force: true });
});

test("imported images deduplicate, release previews and remain available after host restart", async ({ page }, testInfo) => {
  await page.addInitScript(() => {
    const target = window as unknown as Window & { revoked: string[] };
    target.revoked = [];
    const revoke = URL.revokeObjectURL.bind(URL);
    URL.revokeObjectURL = url => { target.revoked.push(url); revoke(url); };
  });
  const first = await control("bootstrap");
  await page.setViewportSize({ width: 320, height: 900 });
  await page.goto(`${origin}/#bootstrap=${first.bootstrap}`);
  await page.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
  await expect(page.getByRole("img", { name: "Selected image 1" })).toBeVisible();
  await expect(page.getByText("Saved images (1)", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Remove image 1" }).click();
  await expect.poll(() => page.evaluate(() => (window as unknown as Window & { revoked: string[] }).revoked.length)).toBeGreaterThan(0);
  await page.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
  await expect(page.getByRole("img", { name: "Selected image 1" })).toBeVisible();
  await expect(page.getByText("Saved images (1)", { exact: true })).toBeVisible();
  await stopHost();
  await startHost();
  const reopened = await control("bootstrap");
  await page.goto(`${origin}/#bootstrap=${reopened.bootstrap}`);
  await page.getByText("Saved images (1)", { exact: true }).click();
  await page.getByRole("button", { name: /Use saved image 1/ }).click();
  await expect(page.getByRole("img", { name: "Selected image 1" })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(320);
  await page.screenshot({ path: testInfo.outputPath("retained-image.png"), fullPage: true });
});

test("bootstrap is removed, credentials stay local and reload reuses the tab session", async ({ page }) => {
  const { bootstrap } = await control("bootstrap");
  const requests: { path: string; authorization?: string; cookie?: string }[] = [];
  page.on("request", request => {
    if (request.url().includes("/api/")) requests.push({ path: new URL(request.url()).pathname, ...request.headers() });
  });
  const errors: string[] = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.goto(`${origin}/?tool=rascal#bootstrap=${bootstrap}`);
  await expect(page.getByRole("status")).toHaveText("Connected locally");
  expect(page.url()).toBe(`${origin}/?tool=rascal`);
  await page.reload();
  await expect(page.getByRole("status")).toHaveText("Connected locally");
  expect(requests.filter(request => request.path === "/api/session")).toHaveLength(1);
  expect(requests.filter(request => request.path === "/api/state").every(request => request.authorization?.startsWith("Bearer ") && !request.cookie)).toBe(true);
  expect(await page.evaluate(() => window.localStorage.length)).toBe(0);
  expect(errors).toEqual([]);
  await page.goto(`${origin}/#bootstrap=${bootstrap}`);
  await expect(page.getByRole("status")).toContainText("expired or was already used");
  expect(page.url()).toBe(`${origin}/`);
});
