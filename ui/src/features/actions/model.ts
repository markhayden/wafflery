export type Json =
  | null
  | boolean
  | number
  | string
  | Json[]
  | { [key: string]: Json };
export type Schema = { [key: string]: Json };
export type ImageSlot = { slot: string; role: string; required: boolean; many?: boolean };
export type Requirement = {
  kind: "config" | "credential" | "env" | "model" | "file" | "dependency";
  name: string;
  state: "missing" | "present" | "valid" | "invalid" | "unknown";
  setup?: string;
  /** Never blocks the action: it serves only the part `for` names (table-manners 3.1). */
  optional?: boolean;
  for?: string;
  /** Needed only when a call picks this choice (table-manners 0.6 draft, 3.1). */
  for_choice?: { param: string; value: string };
  /** The installable that satisfies this requirement (table-manners 0.6 draft, 3.5). */
  installable?: string;
};
export type InstallError = { code: "unavailable" | "busy" | "failed" | "cancelled"; message: string };
/** Something the tool can install on request, such as a model (table-manners 0.6 draft, 3.5). */
export type Installable = {
  id: string;
  title: string;
  kind: "model";
  download_bytes: number;
  installed_bytes: number;
  licence: string;
  state: "installed" | "not_installed" | "installing" | "unavailable";
  progress?: number;
  requirements: Requirement[];
  notes?: string;
  last_outcome: null | { action: "install" | "uninstall"; state: "succeeded" | "failed" | "cancelled"; error: InstallError | null; at: string };
  [extension: `x-${string}`]: Json | undefined;
};
export type Readiness = {
  state: "ready" | "needs_setup" | "unavailable";
  requirements: Requirement[];
};
export type ActivityKind = "task" | "operation" | "utility";
export type Action = {
  kind: ActivityKind;
  group?: string;
  name: string;
  title: string;
  purpose: string;
  params_schema: Schema;
  images?: ImageSlot[];
  ports?: {in: import("./ports").Port[]; out: import("./ports").Port[]};
  locations?: { slot: string; required: boolean; kinds?: string[] }[];
  renders?: { name: string; kind: "server" | "document" }[];
  cost: { may_spend: boolean };
  readiness: Readiness;
  /** A completed result may carry a verdict (table-manners 2.4). */
  answers?: "verdict";
};
export type Declaration = {
  document: {
    contract: string;
    tool: { name: string; version: string; purpose: string };
    actions: Action[];
    vocabulary?: { group?: string[] };
    installables?: Installable[];
  };
  tool_session?: Json;
  revision: string;
  generation: number;
  schemas: Record<string, Schema>;
};
export function object(value: Json | undefined): Schema | undefined {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? value
    : undefined;
}
export function initialParameters(schema: Schema): Schema {
  return Object.fromEntries(
    Object.entries(object(schema.properties) ?? {}).flatMap(([key, value]) => {
      const field = object(value);
      return field && field.default !== undefined
        ? [[key, structuredClone(field.default)]]
        : [];
    }),
  );
}
export function fieldMode(
  schema: Schema,
): "enum" | "string" | "number" | "boolean" | "json" {
  if (schema.oneOf || schema.anyOf || schema.allOf || schema.$ref)
    return "json";
  if (Array.isArray(schema.enum)) return "enum";
  if (schema.type === "string" || schema.type === "boolean") return schema.type;
  if (schema.type === "number" || schema.type === "integer") return "number";
  return "json";
}
export function encodeBindings(
  version: string,
  slots: ImageSlot[],
  selected: Record<string, string[]>,
): Record<string, string[]> {
  if (version !== "0.6.0") throw new Error("Unsupported contract version.");
  const result: Record<string, string[]> = {};
  for (const slot of slots) {
    const value = selected[slot.slot] ?? [];
    if (!slot.many && value.length > 1)
      throw new Error(`${slot.slot} accepts at most one image.`);
    if (slot.required && value.length === 0)
      throw new Error(`${slot.slot} requires ${slot.many ? "at least one" : "one"} image.`);
    if (value.length > 0) result[slot.slot] = [...value];
  }
  if (Object.values(result).reduce((total, images) => total + images.length, 0) > 32)
    throw new Error("An attempt supports at most 32 images.");
  return result;
}
