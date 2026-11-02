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
    if (id === "rascal") {
      args = await rascalServeArgs(join(directory, "data", "Library"), cwd);
    } else {
      for (const name of ["config.toml", "held.json"]) {
        await copyFile(
          fileURLToPath(
            new URL(`../../../testkit/squint/${name}`, import.meta.url),
          ),
          join(cwd, name),
        );
      }
      args = [
        "serve",
        "--config",
        join(cwd, "config.toml"),
        "--provider-script",
        join(cwd, "held.json"),
        "--out",
        join(cwd, "runs"),
      ];
    }
    entries.push({ id, executable, args, cwd, roots: [cwd] });
  }
  return entries;
}
