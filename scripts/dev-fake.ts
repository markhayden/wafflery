import { execFileSync } from "node:child_process";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { root } from "./ui-manifest";

const args = process.argv.slice(2);
if (args.some((argument) => argument !== "--no-open"))
  throw new Error("Usage: bun run scripts/dev-fake.ts [--no-open]");
execFileSync(process.execPath, ["run", "scripts/build-ui.ts"], {
  cwd: root,
  stdio: "inherit",
});
execFileSync(
  "cargo",
  ["build", "--locked", "--bin", "wafflery", "--example", "sample-lab"],
  { cwd: root, stdio: "inherit" },
);
const temporary = await mkdtemp(join(tmpdir(), "wafflery-development-"));
const data = join(temporary, "data"),
  tool = join(temporary, "sample-lab");
await mkdir(data, { mode: 0o700 });
await mkdir(tool, { mode: 0o700 });
await writeFile(
  join(data, "tools.json"),
  JSON.stringify(
    [
      {
        id: "sample",
        executable: join(root, "target/debug/examples/sample-lab"),
        args: [tool, "normal"],
        cwd: tool,
        allow_dirs: [], evidence_dirs: [tool],
      },
    ],
    null,
    2,
  ),
  { mode: 0o600 },
);
console.log(`Isolated development data: ${data}`);
console.log(
  "The sample tool is free, offline and illustrative. It does not repair images.",
);
execFileSync(
  join(root, "target/debug/wafflery"),
  ["--data-dir", data, "--tool", "sample", ...args],
  { cwd: root, stdio: "inherit" },
);
console.log(
  "Closing a tab leaves the host running. Use the Quit power icon and confirm to stop it.",
);
console.log(
  `To reopen this same library, pass --data-dir ${JSON.stringify(data)} --tool sample to target/debug/wafflery.`,
);
