import { expect, test } from "bun:test";
import { mkdtemp, writeFile, rm, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { files, outputs, sameDigests } from "./ui-manifest";

test("detects changed bytes and added or deleted outputs", async () => {
  const directory = await mkdtemp(join(tmpdir(), "wafflehouse-digest-"));
  try {
    await writeFile(join(directory, "index.html"), "original");
    const original = await outputs(directory);
    await writeFile(join(directory, "index.html"), "changed");
    expect(sameDigests(original, await outputs(directory))).toBe(false);
    await writeFile(join(directory, "index.html"), "original");
    expect(sameDigests(original, await outputs(directory))).toBe(true);
    await writeFile(join(directory, "extra.js"), "extra");
    expect(sameDigests(original, await outputs(directory))).toBe(false);
    await rm(join(directory, "index.html"));
    expect(sameDigests(original, await outputs(directory))).toBe(false);
  } finally { await rm(directory, { recursive: true, force: true }); }
});

test("rejects symlinks instead of embedding files outside the asset tree", async () => {
  const directory = await mkdtemp(join(tmpdir(), "wafflehouse-symlink-"));
  try {
    await symlink("/does-not-exist", join(directory, "link"));
    await expect(files(directory)).rejects.toThrow("symlinks");
  } finally { await rm(directory, { recursive: true, force: true }); }
});
