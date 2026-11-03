import { expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, readFile, readdir, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { prepareOfflinePipeline } from "../browser/offline-pipeline";

test("offline demo refuses normal Squint before launch and accepts its testkit executable", async () => {
  const root = await mkdtemp(join(tmpdir(), "wafflery-demo-options-"));
  try {
    await mkdir(join(root, "wrong"));
    await mkdir(join(root, "right"));
    const squint = join(root, "squint");
    await writeFile(squint, "#!/bin/sh\nprintf 'Usage: squint serve [OPTIONS]\\n'\n");
    await chmod(squint, 0o700);
    await expect(prepareOfflinePipeline(join(root, "wrong"), "/usr/bin/true", squint))
      .rejects.toThrow("requires a Squint testkit executable");
    expect(await readdir(join(root, "wrong"))).toEqual([]);
    await writeFile(squint, "#!/bin/sh\nexit 64\n");
    await expect(prepareOfflinePipeline(join(root, "wrong"), "/usr/bin/true", squint))
      .rejects.toThrow("requires a Squint testkit executable");
    await writeFile(squint, "#!/bin/sh\nprintf 'Usage: squint serve [OPTIONS]\\n  --provider-script <PATH>\\n'\n");
    const tools = await prepareOfflinePipeline(join(root, "right"), "/usr/bin/true", squint);
    expect(tools[1]!.executable).toBe(await realpath(squint));
    expect(tools[1]!.args).toContain("--provider-script");
    expect(await readFile(tools[1]!.config!, "utf8")).toContain("fake");
  } finally { await rm(root, { recursive: true, force: true }); }
});
