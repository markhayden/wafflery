import { execFileSync } from "node:child_process";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { root } from "./ui-manifest";
import { prepareOfflinePipeline } from "../ui/tests/browser/offline-pipeline";

const [rascal, squint, ...flags] = process.argv.slice(2);
if (
  !rascal ||
  !squint ||
  flags.length > 1 ||
  flags.some((flag) => flag !== "--no-open")
) {
  throw new Error(
    "Usage: bun run scripts/dev-pipeline.ts /absolute/rascal /absolute/testkit/squint [--no-open]",
  );
}
const temporary = await mkdtemp(join(tmpdir(), "wafflery-pipeline-"));
const data = join(temporary, "data");
const registrations = await prepareOfflinePipeline(temporary, rascal, squint);
execFileSync(process.execPath, ["run", "scripts/build-ui.ts"], {
  cwd: root,
  stdio: "inherit",
});
execFileSync("cargo", ["build", "--locked", "--bin", "wafflery"], {
  cwd: root,
  stdio: "inherit",
});
const source = join(temporary, "registration.json");
await writeFile(source, JSON.stringify(registrations, null, 2), {
  mode: 0o600,
});
const host = join(root, "target/debug/wafflery");
execFileSync(host, ["--register", source, "--data-dir", data], {
  stdio: "inherit",
});
console.log(`Offline test library: ${data}`);
console.log(
  "Squint uses scripted verdicts, not a model judgment. Rascal has no filler. No keys or paid providers are supplied.",
);
console.log(
  "Try Squint / Source fidelity with a candidate and source image. Inspect run → Session shows the saved ledger and run association.",
);
const environment = {
  ...process.env,
  WAFFLERY_BIN: host,
  WAFFLERY_DATA_DIR: data,
};
if (flags.includes("--no-open")) {
  execFileSync(host, ["--tool", "squint", "--no-open"], {
    env: environment,
    stdio: "inherit",
  });
} else {
  // H1 stays unchanged: the tool invokes exactly --tool squint.
  execFileSync(
    registrations.find((entry) => entry.id === "squint")!.executable,
    ["playground"],
    { env: environment, stdio: "inherit" },
  );
}
const quote = (value: string) => `'${value.replaceAll("'", "'\\''")}'`;
console.log("Reopen this same library:");
console.log(
  `WAFFLERY_BIN=${quote(host)} WAFFLERY_DATA_DIR=${quote(data)} ${quote(squint)} playground`,
);
console.log(
  "Use the Quit power icon and confirm to stop this host. Closing a tab leaves it running. macOS may eventually remove this temporary library.",
);
