import { afterEach, expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { requireBrowserBinaries } from "../browser/binaries";

const directories: string[] = [];
afterEach(async () => {
  await Promise.all(directories.splice(0).map(path => rm(path, { recursive: true, force: true })));
});
async function fixture() {
  const root = await mkdtemp(join(tmpdir(), "wafflery-browser-binaries-"));
  directories.push(root);
  await mkdir(join(root, "target/debug/examples"), { recursive: true });
  return root;
}
async function executable(path: string) {
  await writeFile(path, "#!/bin/sh\nexit 0\n");
  await chmod(path, 0o700);
}

test("missing host fails before workers with an actionable build command", async () => {
  const root = await fixture();
  await expect(requireBrowserBinaries(root)).rejects.toThrow("wafflery");
  await expect(requireBrowserBinaries(root)).rejects.toThrow("bun run --cwd ui test:browser");
});

test("missing peer is reported even when the host exists", async () => {
  const root = await fixture();
  await executable(join(root, "target/debug/wafflery"));
  await expect(requireBrowserBinaries(root)).rejects.toThrow("sample-lab");
});

test("directories and non-executable files are not accepted as binaries", async () => {
  const root = await fixture();
  const host = join(root, "target/debug/wafflery");
  await mkdir(host);
  await expect(requireBrowserBinaries(root)).rejects.toThrow("wafflery");
  await rm(host, { recursive: true });
  await writeFile(host, "not executable", { mode: 0o600 });
  await expect(requireBrowserBinaries(root)).rejects.toThrow("wafflery");
});

test("both built executables pass without running or changing them", async () => {
  const root = await fixture();
  await executable(join(root, "target/debug/wafflery"));
  await executable(join(root, "target/debug/examples/sample-lab"));
  await requireBrowserBinaries(root);
});
