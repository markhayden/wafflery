import { constants } from "node:fs";
import { access, stat } from "node:fs/promises";
import { resolve } from "node:path";

// Build once in test:browser, before parallel workers. Fixtures must never rebuild these paths.
export async function requireBrowserBinaries(repository = resolve("..")): Promise<void> {
  for (const relative of ["target/debug/wafflery", "target/debug/examples/sample-lab"]) {
    const path = resolve(repository, relative);
    try {
      if (!(await stat(path)).isFile()) throw new Error("not a regular file");
      await access(path, constants.X_OK);
    } catch (cause) {
      throw new Error(
        `Browser-test executable is missing or unusable: ${path}. Run bun run --cwd ui test:browser to build the host and peer before starting workers.`,
        { cause },
      );
    }
  }
}

export default async function setup(): Promise<void> {
  await requireBrowserBinaries();
}
