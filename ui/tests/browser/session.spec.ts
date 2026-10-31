import { test, expect } from "@playwright/test";
import { spawn, type ChildProcess } from "node:child_process";
import { mkdtemp, realpath, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createHash } from "node:crypto";
import { createConnection } from "node:net";

let child: ChildProcess;
let temporary: string;
let root: string;
let origin: string;

// Test-only private IPC client; production browser code never accesses host discovery.
async function control(operation: string): Promise<{ bootstrap: string }> {
  const canonical = await realpath(root);
  const hash = createHash("sha256").update(canonical).digest("hex").slice(0, 32);
  return new Promise((resolveReply, reject) => {
    const socket = createConnection(join(tmpdir(), `wh-${process.getuid?.()}`, `${hash}.sock`));
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

test.beforeAll(async () => {
  temporary = await mkdtemp(join(tmpdir(), "wafflehouse-browser-"));
  root = join(temporary, "data");
  child = spawn(resolve("../target/debug/wafflehouse"), ["--foreground", "--no-open", "--data-dir", root], { stdio: ["ignore", "pipe", "pipe"] });
  origin = await new Promise<string>((resolveOrigin, reject) => {
    child.once("error", reject);
    child.once("exit", () => reject(new Error("host exited before readiness")));
    child.stdout?.once("data", chunk => resolveOrigin(String(chunk).trim()));
  });
  await expect.poll(async () => { try { await control("status"); return true; } catch { return false; } }).toBe(true);
});

test.afterAll(async () => {
  if (child && child.exitCode === null) {
    const exited = new Promise<void>(resolveExit => child.once("exit", () => resolveExit()));
    child.kill("SIGTERM");
    await exited;
  }
  if (temporary) await rm(temporary, { recursive: true, force: true });
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
