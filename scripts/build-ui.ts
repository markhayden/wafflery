import { mkdtemp, rename, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { build, inputs, outputs, root, sameDigests, type Manifest } from "./ui-manifest";

const temporary = await mkdtemp(join(root, "ui/.prebuilt-"));
try {
  const before = await inputs();
  await build(temporary);
  if (!sameDigests(before, await inputs())) throw new Error("UI inputs changed during the build; retry.");
  const manifest: Manifest = { version: 1, inputs: before, outputs: await outputs(temporary) };
  await writeFile(join(temporary, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  await rm(join(root, "ui/prebuilt"), { recursive: true, force: true });
  await rename(temporary, join(root, "ui/prebuilt"));
  console.log("Published prebuilt UI and SHA-256 manifest.");
} finally {
  await rm(temporary, { recursive: true, force: true });
}
