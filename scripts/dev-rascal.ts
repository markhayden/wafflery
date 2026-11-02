import { rascalServeArgs } from "../ui/tests/browser/rascal-args";
import { execFileSync } from "node:child_process";
import { access, mkdir, mkdtemp, realpath, writeFile } from "node:fs/promises";
import { constants } from "node:fs";
import { tmpdir } from "node:os";
import { isAbsolute, join } from "node:path";
import { root } from "./ui-manifest";

const [selected, ...flags] = process.argv.slice(2);
if (
  !selected ||
  !isAbsolute(selected) ||
  flags.length > 1 ||
  flags.some((flag) => flag !== "--no-open")
) {
  throw new Error(
    "Usage: bun run scripts/dev-rascal.ts /absolute/path/to/rascal [--no-open]",
  );
}
const executable = await realpath(selected);
await access(executable, constants.X_OK);
execFileSync(process.execPath, ["run", "scripts/build-ui.ts"], {
  cwd: root,
  stdio: "inherit",
});
execFileSync("cargo", ["build", "--locked", "--bin", "wafflery"], {
  cwd: root,
  stdio: "inherit",
});
const temporary = await mkdtemp(join(tmpdir(), "wafflery-rascal-"));
const data = join(temporary, "data"),
  tool = join(temporary, "rascal");
await mkdir(tool, { mode: 0o700 });
const registration = join(temporary, "registration.json");
await writeFile(
  registration,
  JSON.stringify(
    [
      {
        id: "rascal",
        executable,
        args: rascalServeArgs(tool),
        cwd: tool,
        allow_dirs: [], evidence_dirs: [tool],
      },
    ],
    null,
    2,
  ),
  { mode: 0o600 },
);
const host = join(root, "target/debug/wafflery");
execFileSync(host, ["--register", registration, "--data-dir", data], {
  stdio: "inherit",
});
console.log(`Rascal library: ${data}`);
console.log(
  "No provider config or credentials are supplied. Choose Heal a seam; set image coordinates and projection to equirectangular for a wrap-edge mask.",
);
const environment = {
  ...process.env,
  WAFFLERY_BIN: host,
  WAFFLERY_DATA_DIR: data,
};
if (flags.includes("--no-open")) {
  execFileSync(host, ["--tool", "rascal", "--no-open"], {
    env: environment,
    stdio: "inherit",
  });
} else {
  // Exercise the actual H1 entry point; Rascal passes only --tool rascal.
  execFileSync(executable, ["playground"], {
    env: environment,
    stdio: "inherit",
  });
}
const quote = (value: string) => `'${value.replaceAll("'", "'\\''")}'`;
console.log("Reopen this same library through Rascal:");
console.log(
  `WAFFLERY_BIN=${quote(host)} WAFFLERY_DATA_DIR=${quote(data)} ${quote(executable)} playground`,
);
console.log(
  "Closing a tab leaves the host running. Use the Quit power icon and confirm to stop it. This temporary library may eventually be removed by macOS.",
);
