import { execFileSync } from "node:child_process";
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
  const installed = await Promise.all(([
    ["rascal", rascal], ["squint", squint],
  ] as const).map(async ([id, selected]) => {
    if (!selected || !isAbsolute(selected))
      throw new Error(`Select an absolute ${id} executable`);
    const executable = await realpath(selected);
    await access(executable, constants.X_OK);
    return { id, executable };
  }));
  const setup = "The offline demo requires a Squint testkit executable (built with --features testkit). Choose the testkit binary instead of a normal Squint build.";
  let help: string;
  try {
    help = execFileSync(installed[1]!.executable, ["serve", "--help"], {
      encoding: "utf8", timeout: 5000, maxBuffer: 1024 * 1024,
    });
  } catch (cause) { throw new Error(setup, { cause }); }
  if (!help.includes("--provider-script")) throw new Error(setup);
  const entries = [];
  for (const { id, executable } of installed) {
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
