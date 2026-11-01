import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { build, inputs, outputs, root, sameDigests, type Manifest } from "./ui-manifest";

const manifest: Manifest = JSON.parse(await readFile(join(root, "ui/prebuilt/manifest.json"), "utf8"));
if (manifest.version !== 1 || !sameDigests(manifest.inputs, await inputs())) {
  throw new Error("Prebuilt UI inputs are stale. Run bun run scripts/build-ui.ts.");
}
if (!sameDigests(manifest.outputs, await outputs(join(root, "ui/prebuilt")))) {
  throw new Error("Prebuilt UI outputs do not match their manifest.");
}
const temporary = await mkdtemp(join(tmpdir(), "wafflery-ui-check-"));
try {
  for (const pass of ["first", "second"]) {
    const directory = join(temporary, pass);
    await build(directory);
    if (!sameDigests(manifest.outputs, await outputs(directory))) {
      throw new Error(`${pass} clean build differs from prebuilt assets.`);
    }
  }
  if (!sameDigests(manifest.inputs, await inputs())) throw new Error("UI inputs changed during verification.");
  console.log("Inputs and outputs match; two clean UI builds are byte-identical.");
} finally {
  await rm(temporary, { recursive: true, force: true });
}
