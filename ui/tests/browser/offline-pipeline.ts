import { rascalServeArgs } from "./rascal-args";
import { access, copyFile, mkdir, realpath } from "node:fs/promises";
import { constants } from "node:fs";
import { isAbsolute, join } from "node:path";
import { fileURLToPath } from "node:url";

// Development/test registration only. Production discovery never knows tool-specific flags.
export async function prepareOfflinePipeline(
  directory: string,
  rascal: string,
  squint: string,
  referenceCheck = false,
) {
  const entries = [];
  for (const [id, selected] of [
    ["rascal", rascal],
    ["squint", squint],
  ] as const) {
    if (!selected || !isAbsolute(selected))
      throw new Error(`Select an absolute ${id} executable`);
    const executable = await realpath(selected);
    await access(executable, constants.X_OK);
    const cwd = join(directory, id);
    await mkdir(cwd, { mode: 0o700 });
    let args: string[];
    let config: string | undefined;
    if (id === "rascal") {
      args = rascalServeArgs(cwd);
    } else {
      for (const name of ["config.toml", referenceCheck ? "references-held.json" : "held.json"]) {
        await copyFile(
          fileURLToPath(
            new URL(`../../../testkit/squint/${name}`, import.meta.url),
          ),
          join(cwd, name),
        );
      }
      config = join(cwd, "config.toml");
      args = [
        "serve",
        "--provider-script",
        join(cwd, referenceCheck ? "references-held.json" : "held.json"),
        "--out",
        join(cwd, "runs"),
      ];
    }
    entries.push({ id, executable, args, cwd, allow_dirs: [], evidence_dirs: [cwd], ...(config ? { config } : {}) });
  }
  return entries;
}
