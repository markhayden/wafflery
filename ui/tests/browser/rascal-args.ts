import { join } from "node:path";

// Development/test descriptors only. The generic host never invents tool-specific flags.
export function rascalServeArgs(tool: string): string[] {
  return [
    "serve",
    "--workspace", join(tool, "workspace"), "--runs", join(tool, "runs"),
  ];
}
