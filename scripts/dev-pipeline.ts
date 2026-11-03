import { execFileSync, spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { root } from "./ui-manifest";
import { pipelineOptions } from "./dev-pipeline-options";
import { prepareOfflinePipeline } from "../ui/tests/browser/offline-pipeline";

const [rascal, squint, ...flags] = process.argv.slice(2);
if (!rascal || !squint) {
  throw new Error(
    "Usage: bun run scripts/dev-pipeline.ts /absolute/rascal /absolute/testkit/squint [--start] [--no-open] [--library <absolute dir>] [--register-only]",
  );
}
const options = pipelineOptions(flags);
// A persistent library keeps its tool setup and saved work between runs; the setup is made once.
const temporary = options.library ?? await mkdtemp(join(tmpdir(), "wafflery-pipeline-"));
const data = join(temporary, "data");
const setup = join(temporary, "setup");
const source = join(temporary, "registration.json");
let registrations: Awaited<ReturnType<typeof prepareOfflinePipeline>>;
if (options.library && existsSync(source)) {
  registrations = JSON.parse(await readFile(source, "utf8"));
} else {
  await mkdir(setup, { recursive: true, mode: 0o700 });
  registrations = await prepareOfflinePipeline(setup, rascal, squint);
}
// The dev UI server serves the UI source directly; only a packaged host needs the built UI.
if (!options.registerOnly) {
  execFileSync(process.execPath, ["run", "scripts/build-ui.ts"], {
    cwd: root,
    stdio: "inherit",
  });
}
execFileSync("cargo", ["build", "--locked", "--bin", "wafflery"], {
  cwd: root,
  stdio: "inherit",
});
await writeFile(source, JSON.stringify(registrations, null, 2), {
  mode: 0o600,
});
const host = join(root, "target/debug/wafflery");
execFileSync(host, ["--register", source, "--data-dir", data], {
  stdio: "inherit",
});
console.log(`Offline test library: ${data}`);
if (options.registerOnly) process.exit(0);
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
if (options.start) {
  console.log(
    "Ctrl-C stops this host and its tools. Closing a browser tab leaves it running.",
  );
  const child = spawn(
    host,
    ["start", "--data-dir", data, ...(options.noOpen ? ["--no-open"] : [])],
    { env: environment, stdio: "inherit" },
  );
  // Also forward signals sent only to the script, rather than the whole terminal group.
  const interrupt = () => { child.kill("SIGINT"); };
  const terminate = () => { child.kill("SIGTERM"); };
  process.on("SIGINT", interrupt);
  process.on("SIGTERM", terminate);
  try {
    process.exitCode = await new Promise<number>((resolve, reject) => {
      child.once("error", reject);
      child.once("exit", (code, signal) =>
        resolve(code ?? (signal === "SIGINT" ? 130 : signal === "SIGTERM" ? 143 : 1)),
      );
    });
  } finally {
    process.off("SIGINT", interrupt);
    process.off("SIGTERM", terminate);
  }
} else if (options.noOpen) {
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
if (!options.start) {
  const quote = (value: string) => `'${value.replaceAll("'", "'\\''")}'`;
  console.log("Reopen this same library:");
  console.log(
    `WAFFLERY_BIN=${quote(host)} WAFFLERY_DATA_DIR=${quote(data)} ${quote(squint)} playground`,
  );
  console.log(
    "Use the Quit power icon and confirm to stop this host. Closing a tab leaves it running. macOS may eventually remove this temporary library.",
  );
}
