import type { LocalApi } from "./client";
export type ToolSession = {
  contract: "0.4.0";
  tool: { name: string; version: string };
  id: string;
  started: string;
  budget_usd: number | null;
  spent_usd: number;
  unresolved_usd: number;
  reserved_usd: number;
  remaining_usd: number | null;
  allowed_dirs: string[];
};
const amount = (value: unknown): value is number =>
  typeof value === "number" && Number.isFinite(value) && value >= 0;
const record = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === "object" && !Array.isArray(value);
// Structural display check only: the host validates canonical filesystem identity and grants.
const directories = (value: unknown): value is string[] =>
  Array.isArray(value) && new Set(value).size === value.length && value.every(path =>
    typeof path === "string" && !path.includes("\0") &&
    (path === "/" || path.startsWith("/") && path.slice(1).split("/").every(part => part !== "" && part !== "." && part !== "..")));
export function parseToolSession(value: unknown): ToolSession {
  if (!record(value) || value.contract !== "0.4.0" || !record(value.tool) ||
      typeof value.tool.name !== "string" || typeof value.tool.version !== "string" ||
      typeof value.id !== "string" || !/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(value.id) ||
      typeof value.started !== "string" || !Number.isFinite(Date.parse(value.started)) ||
      !directories(value.allowed_dirs) ||
      !amount(value.spent_usd) || !amount(value.reserved_usd) || !amount(value.unresolved_usd) ||
      value.unresolved_usd > value.spent_usd + 1e-6 ||
      !(value.budget_usd === null && value.remaining_usd === null ||
        amount(value.budget_usd) && amount(value.remaining_usd) &&
        Math.abs(value.remaining_usd - Math.max(0, value.budget_usd - value.spent_usd - value.reserved_usd)) <= 1e-6)) {
    throw new Error("The tool returned invalid session information. Its current allowance and folder access are unavailable.");
  }
  return value as ToolSession;
}
export async function readToolSession(api: LocalApi, tool: string): Promise<ToolSession> {
  const response = await api.request(`/api/tools/${encodeURIComponent(tool)}/session`);
  return parseToolSession(await response.json());
}
export function usd(value: number): string {
  if (value > 0 && value < 0.000001) return "< $0.000001";
  return new Intl.NumberFormat("en-US", {
    style: "currency", currency: "USD", minimumFractionDigits: 2, maximumFractionDigits: 6,
  }).format(value);
}
