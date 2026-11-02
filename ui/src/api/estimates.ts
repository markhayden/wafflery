import type { LocalApi } from "./client";
import { isReadiness, type Submission } from "./workspace";
import type { Readiness } from "../features/actions/model";
import { parseToolSession, type ToolSession } from "./tool-session";

export type Preparation = {
  id: string; tool: string; action: string; revision: string; expires_ms: number; bytes: number;
};
export type Estimate = {
  effective: { model: string | null; config_identity: string | null };
  worst_case_usd: number | null;
  cap_usd: number | null;
  allowed: boolean;
  refused: "readiness" | "cap" | "budget" | null;
  readiness: Readiness;
  session: ToolSession;
  expires: string;
};
export type PreparedEstimate = { preparation: Preparation; estimate: Estimate };
const record = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === "object" && !Array.isArray(value);
const amount = (value: unknown): value is number =>
  typeof value === "number" && Number.isFinite(value) && value >= 0;

/** Keep incomplete editor text intact; only explicit positive decimal values become a cap. */
export function positiveCap(value: unknown): number | undefined {
  if (typeof value === "string") {
    const text = value.trim();
    if (!/^(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/.test(text)) return undefined;
    value = Number(text);
  }
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? value : undefined;
}
function parsePreparation(value: unknown): Preparation {
  if (!record(value) || !["id", "tool", "action", "revision"].every(key => typeof value[key] === "string" && value[key].length > 0) ||
      !Number.isSafeInteger(value.expires_ms) || !amount(value.expires_ms) ||
      !Number.isSafeInteger(value.bytes) || !amount(value.bytes))
    throw new Error("The prepared inputs could not be identified.");
  return value as Preparation;
}
export function parseEstimate(value: unknown): PreparedEstimate {
  if (!record(value) || !record(value.estimate)) throw new Error("The estimate could not be read.");
  const preparation = parsePreparation(value.preparation), estimate = value.estimate;
  if (!record(estimate.effective) ||
      !(estimate.effective.model === null || typeof estimate.effective.model === "string") ||
      !(estimate.effective.config_identity === null || typeof estimate.effective.config_identity === "string" && /^sha256:[a-f0-9]{64}$/.test(estimate.effective.config_identity)) ||
      !(estimate.worst_case_usd === null || amount(estimate.worst_case_usd)) ||
      !(estimate.cap_usd === null || typeof estimate.cap_usd === "number" && positiveCap(estimate.cap_usd) !== undefined) ||
      typeof estimate.allowed !== "boolean" ||
      !(estimate.refused === null || typeof estimate.refused === "string" && ["readiness", "cap", "budget"].includes(estimate.refused)) ||
      !isReadiness(estimate.readiness) ||
      typeof estimate.expires !== "string" || !Number.isFinite(Date.parse(estimate.expires)))
    throw new Error("The estimate contains invalid cost or setup information.");
  const session = parseToolSession(estimate.session);
  if (estimate.allowed !== (estimate.refused === null) ||
      (estimate.refused === "readiness" ? estimate.readiness.state === "ready" :
        estimate.readiness.state !== "ready" || estimate.worst_case_usd === null ||
        estimate.cap_usd === null || estimate.effective.config_identity === null) ||
      (estimate.allowed && (Number(estimate.worst_case_usd) > Number(estimate.cap_usd) + 1e-6 ||
        session.remaining_usd !== null && Number(estimate.worst_case_usd) > session.remaining_usd + 1e-6)))
    throw new Error("The estimate contradicts its cost or setup status.");
  // The server retains the binding. This view never supplies spending authorization.
  return { preparation, estimate: { ...estimate, session } as Estimate };
}
export function estimateDeadline(value: PreparedEstimate): number {
  return Math.min(value.preparation.expires_ms, Date.parse(value.estimate.expires));
}
const jsonPost = (value: unknown): RequestInit => ({
  method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(value),
});
export function estimateClient(api: LocalApi) {
  async function readQuote(receipt: Preparation, session: string, init?: RequestInit): Promise<PreparedEstimate> {
    const quote = parseEstimate(await (await api.request(`/api/preparations/${encodeURIComponent(receipt.id)}/estimate`, init)).json());
    if (!(["id", "tool", "action", "revision", "expires_ms", "bytes"] as const).every(key => quote.preparation[key] === receipt[key]) || quote.estimate.session.id !== session)
      throw new Error("The estimate belongs to different inputs or a different tool session. Estimate again.");
    return quote;
  }
  return {
    async prepare(request: Submission): Promise<Preparation> {
      const receipt = parsePreparation(await (await api.request("/api/preparations", jsonPost(request))).json());
      if (!(["id", "tool", "action", "revision"] as const).every(key => receipt[key] === request[key]))
        throw new Error("The preparation belongs to different inputs.");
      return receipt;
    },
    estimate: (receipt: Preparation, session: string) => readQuote(receipt, session, jsonPost({})),
    read: (receipt: Preparation, session: string) => readQuote(receipt, session),
    async discard(id: string): Promise<void> {
      await api.request(`/api/preparations/${encodeURIComponent(id)}`, { method: "DELETE" });
    },
  };
}
