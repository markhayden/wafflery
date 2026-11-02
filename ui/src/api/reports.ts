import { LocalApi } from "./client";
export type ReportView = {
  id: string; attempt: string; name: string; kind: "server" | "document";
  phase: "starting" | "ready" | "stopping" | "needs_force" | "exited";
  exit_code: number | null; error: string | null; idle_expired: boolean;
  diagnostics: string; diagnostics_truncated: boolean;
};
export interface ReportPort {
  current(attempt: string): Promise<ReportView | null>;
  open(attempt: string, name: string): Promise<ReportView>;
  read(id: string): Promise<ReportView>;
  close(id: string): Promise<ReportView>;
  force(id: string): Promise<ReportView>;
  activity(id: string): Promise<ReportView>;
  frame(id: string): Promise<string>;
}
function object(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
export function parseReport(value: unknown): ReportView {
  if (!object(value) || typeof value.id !== "string" || !/^[a-f0-9]{32}$/.test(value.id)
    || typeof value.attempt !== "string" || !value.attempt || typeof value.name !== "string" || !value.name
    || typeof value.kind !== "string" || !["server", "document"].includes(value.kind)
    || typeof value.phase !== "string" || !["starting", "ready", "stopping", "needs_force", "exited"].includes(value.phase)
    || !(value.exit_code === null || (typeof value.exit_code === "number" && Number.isSafeInteger(value.exit_code)))
    || !(value.error === null || typeof value.error === "string")
    || typeof value.idle_expired !== "boolean" || typeof value.diagnostics !== "string"
    || typeof value.diagnostics_truncated !== "boolean") throw new Error("Invalid report observation.");
  return value as ReportView;
}
export function reportClient(api: LocalApi): ReportPort {
  const post = (body: unknown): RequestInit => ({ method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
  const path = (id: string) => `/api/views/${encodeURIComponent(id)}`;
  const attemptPath = (id: string) => `/api/attempts/${encodeURIComponent(id)}/views`;
  const observed = async (id: string, suffix = "", init?: RequestInit) => {
    const view = parseReport(await (await api.request(path(id) + suffix, init)).json());
    if (view.id !== id) throw new Error("Report identity changed.");
    return view;
  };
  return {
    async current(attempt) {
      const value: unknown = await (await api.request(attemptPath(attempt))).json();
      if (!object(value)) throw new Error("Invalid report lookup.");
      if (value.view === null) return null;
      const view = parseReport(value.view);
      if (view.attempt !== attempt) throw new Error("Report belongs to another saved run.");
      return view;
    },
    async open(attempt, name) {
      const view = parseReport(await (await api.request(attemptPath(attempt), post({name}))).json());
      if (view.attempt !== attempt || view.name !== name) throw new Error("Report selection changed.");
      return view;
    },
    read: (id) => observed(id),
    close: (id) => observed(id, "/close", post({})),
    force: (id) => observed(id, "/force-close", post({confirmed:true})),
    activity: (id) => observed(id, "/activity", post({})),
    async frame(id) {
      const value: unknown = await (await api.request(`${path(id)}/frame`, post({}))).json();
      if (!object(value) || typeof value.url !== "string" || !/^\/views\/[a-f0-9]{64}$/.test(value.url))
        throw new Error("Invalid report frame URL.");
      return value.url;
    },
  };
}
