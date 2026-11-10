import type { Json, Schema } from "../actions/model";

export type InvocationView =
  | { state: "resolved"; value: Schema }
  | { state: "unresolved" }
  | { state: "unavailable" };

function record(value: Json | undefined): value is Schema {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

/** Read saved result evidence only; never derive an invocation from a request or current draft. */
export function savedInvocation(result: Json): InvocationView {
  if (!record(result) || result.contract !== "0.6.0" || !Object.hasOwn(result, "invocation"))
    return { state: "unavailable" };
  const invocation = result.invocation;
  if (invocation === null && result.status === "invalid") return { state: "unresolved" };
  if (!record(invocation) || typeof invocation.action !== "string" || !invocation.action ||
    !record(invocation.arguments) || !record(invocation.effective) ||
    !Array.isArray(invocation.requirements) || !invocation.requirements.every(name => typeof name === "string"))
    return { state: "unavailable" };
  return { state: "resolved", value: invocation };
}
