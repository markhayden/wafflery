import { mkdir, realpath } from "node:fs/promises";
import { join } from "node:path";

// Development/test descriptors only. The generic host never invents tool-specific flags.
export async function rascalServeArgs(library: string, tool: string): Promise<string[]> {
  const inputs = join(library, "execution");
  await mkdir(inputs, { recursive: true, mode: 0o700 });
  return [
    "serve", "--allow-dir", await realpath(inputs),
    "--workspace", join(tool, "workspace"), "--runs", join(tool, "runs"),
  ];
}
