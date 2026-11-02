/** Host permissions observed for this child; effective input access is reported by its session. */
export type DirectoryGrants = {
  requested_dirs: string[];
  evidence_dirs: string[];
};
function directories(value: unknown): value is string[] {
  return Array.isArray(value) && value.length <= 32 && value.every(path =>
    typeof path === "string" && !path.includes("\0") &&
    (path === "/" || path.startsWith("/") && path.slice(1).split("/").every(part => part !== "" && part !== "." && part !== "..")));
}
export function parseDirectoryGrants(value: unknown): DirectoryGrants | null {
  if (value === null) return null;
  if (typeof value !== "object" || Array.isArray(value) || value === undefined ||
      Object.keys(value).length !== 2 || !("requested_dirs" in value) || !("evidence_dirs" in value) ||
      !directories(value.requested_dirs) || !directories(value.evidence_dirs))
    throw new Error("The tool's registered folders could not be read.");
  return { requested_dirs: [...value.requested_dirs], evidence_dirs: [...value.evidence_dirs] };
}
