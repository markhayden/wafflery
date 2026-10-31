import { createHash } from "node:crypto";
import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";

export type Digests = Record<string, string>;
export type Manifest = { version: 1; inputs: Digests; outputs: Digests };
export const root = join(import.meta.dir, "..");

export async function files(directory: string, prefix = ""): Promise<string[]> {
  const result: string[] = [];
  for (const entry of await readdir(join(directory, prefix), { withFileTypes: true })) {
    const name = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isSymbolicLink()) throw new Error(`UI build does not accept symlinks: ${name}`);
    if (entry.isDirectory()) result.push(...await files(directory, name));
    else if (entry.isFile()) result.push(name);
  }
  return result.sort();
}

export async function digests(directory: string, names: string[]): Promise<Digests> {
  const entries = await Promise.all([...names].sort().map(async name => [
    name, createHash("sha256").update(await readFile(join(directory, name))).digest("hex"),
  ]));
  return Object.fromEntries(entries);
}

export async function inputs(directory = root): Promise<Digests> {
  const sources = (await files(join(directory, "ui/src"))).map(name => `ui/src/${name}`);
  return digests(directory, [...sources,
    "ui/index.html", "ui/package.json", "ui/bun.lock", "ui/tsconfig.json", "ui/vite.config.ts",
    "scripts/build-ui.ts", "scripts/check-ui.ts", "scripts/ui-manifest.ts",
  ]);
}

export async function outputs(directory: string): Promise<Digests> {
  return digests(directory, (await files(directory)).filter(name => name !== "manifest.json"));
}

export function sameDigests(left: Digests, right: Digests): boolean {
  const keys = Object.keys(left).sort();
  return keys.length === Object.keys(right).length && keys.every(key => left[key] === right[key]);
}

export async function build(directory: string): Promise<void> {
  const process = Bun.spawn(["bun", "run", "build", "--outDir", directory, "--emptyOutDir"], {
    cwd: join(root, "ui"), stdout: "inherit", stderr: "inherit",
  });
  if (await process.exited !== 0) throw new Error("UI build failed; existing prebuilt assets were preserved.");
}
