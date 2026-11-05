import { parseDirectoryGrants, type DirectoryGrants } from "./directory-grants";
import { LocalApi } from "./client";
import { isAsset, type Asset } from "./assets";
import type { Declaration, Json, Readiness, Schema } from "../features/actions/model";
import type { Coordinates } from "../components/comparison";
/** A saved image: its role and, when the tool gave one, the label a person sees in its place. */
export type SavedImage = { asset: Asset; image: Coordinates; role: string; label?: string };
export type BindingOrigin=({attempt:string}|{example:string})&{port:string;occurrence:number};
export type InputBinding = (
  | {kind: "image"; asset: string; frame?: {projection:"flat"|"equirectangular";topology:"bounded"|"wrap"}}
  | {kind: "artifact"; reference: Schema}
  | {kind: "location"; value: Schema}) & {origin?:BindingOrigin};
export type ReplayContext={id:string;generation:number};
export type BundleSource={id:string;step:number;replay?:ReplayContext};
export type Submission = {
  id: string;
  tool: string;
  action: string;
  revision: string;
  params: Schema;
  images: Record<string, string[]>;
  bindings?: Record<string, InputBinding[]>;
  lineage?:Record<string,(BindingOrigin|null)[]>;
  source_bundle?:BundleSource;
  frames?: Record<
    string,
    { projection: "flat" | "equirectangular"; topology: "bounded" | "wrap" }[]
  >;
};
export type Attempt = {
  id: string;
  sequence: number;
  created_ms: number;
  request: Submission;
  arguments: Json;
  declaration: Declaration;
  inputs: Record<string, SavedImage[]>;
  wire: Json;
  raw_response: Json;
  observations?: Json;
  result: Json;
  execution: string;
  preservation: { contract: string; native: string; report: string };
  outputs: SavedImage[];
  journal: Json;
  notices: string[];
};
export type RegisteredTool = {
  id: string;
  directory_grants: DirectoryGrants | null;
  declaration: Declaration | null;
  error: string | null;
  busy: boolean;
  progress?: { message?: string } | null;
};
export type AttemptSummary = {
  id: string;
  sequence: number;
  created_ms: number;
  tool: string;
  action: string;
  execution: string;
};
export type Snapshot = {
  event_session?: string;
  tools: RegisteredTool[];
  attempts: AttemptSummary[];
};
export type HistoryPage = { items: AttemptSummary[]; cursor: number | null };
function summary(value: unknown): value is AttemptSummary {
  return (
    record(value) &&
    typeof value.id === "string" &&
    typeof value.sequence === "number" &&
    typeof value.tool === "string" &&
    typeof value.action === "string" &&
    typeof value.execution === "string"
  );
}
function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
export function isReadiness(value: unknown): value is Readiness {
  return record(value) && typeof value.state === "string" && ["ready", "needs_setup", "unavailable"].includes(value.state) &&
    Array.isArray(value.requirements) && value.requirements.every((item: unknown) =>
      record(item) && typeof item.name === "string" && item.name.length > 0 &&
      typeof item.kind === "string" && ["config", "credential", "env", "model", "file", "dependency"].includes(item.kind) &&
      typeof item.state === "string" && ["missing", "present", "valid", "invalid", "unknown"].includes(item.state) &&
      (item.setup === undefined || typeof item.setup === "string") &&
      (item.optional === undefined || typeof item.optional === "boolean") &&
      (item.for === undefined || (item.optional === true && typeof item.for === "string")));
}
function declaration(value: unknown): value is Declaration {
  if (
    !record(value) ||
    typeof value.revision !== "string" ||
    !record(value.document) ||
    value.document.contract !== "0.4.0" ||
    !record(value.document.tool) ||
    typeof value.document.tool.name !== "string" ||
    !Array.isArray(value.document.actions) ||
    (value.document.vocabulary !== undefined && (!record(value.document.vocabulary) ||
      (value.document.vocabulary.group !== undefined && (!Array.isArray(value.document.vocabulary.group) ||
        !value.document.vocabulary.group.every((group: unknown) => typeof group === "string")))))
  )
    return false;
  return value.document.actions.every(
    (action: unknown) =>
      record(action) &&
      (action.kind === "task" || action.kind === "operation" || action.kind === "utility") &&
      (action.group === undefined || typeof action.group === "string") &&
      typeof action.name === "string" &&
      typeof action.title === "string" &&
      typeof action.purpose === "string" &&
      (action.ports===undefined || (record(action.ports) && [action.ports.in,action.ports.out].every(ports=>Array.isArray(ports)&&ports.every((port:unknown)=>record(port)&&typeof port.name==="string"&&typeof port.type==="string"&&(port.required===undefined||typeof port.required==="boolean")&&(port.many===undefined||typeof port.many==="boolean"))))) &&
      record(action.params_schema) &&
      record(action.cost) &&
      typeof action.cost.may_spend === "boolean" &&
      isReadiness(action.readiness) &&
      (action.renders === undefined || (Array.isArray(action.renders) && action.renders.every((render: unknown) =>
        record(render) && typeof render.name === "string" && render.name.length > 0 &&
        (render.kind === "server" || render.kind === "document")))),
  );
}
function savedImage(value: unknown): value is SavedImage {
  return (
    record(value) &&
    isAsset(value.asset) &&
    record(value.image) &&
    typeof value.image.width === "number" &&
    typeof value.image.height === "number" &&
    typeof value.image.projection === "string" &&
    typeof value.image.topology === "string" &&
    typeof value.role === "string" &&
    (value.label === undefined || typeof value.label === "string")
  );
}
function normalizedSubmission(request: Submission) {
  return {
    ...request,
    frames: request.frames === undefined ? {} : request.frames,
    bindings: request.bindings === undefined ? {} : request.bindings,
    lineage: request.lineage === undefined ? {} : request.lineage,
  };
}
function sameJson(left: unknown, right: unknown): boolean {
  if (left === right) return true;
  if (Array.isArray(left)) return Array.isArray(right) && left.length === right.length &&
    left.every((item,index) => sameJson(item,right[index]));
  return record(left) && record(right) && Object.keys(left).length === Object.keys(right).length &&
    Object.keys(left).every(key => Object.hasOwn(right,key) && sameJson(left[key],right[key]));
}
function attempt(value: unknown): value is Attempt {
  return (
    record(value) &&
    typeof value.id === "string" &&
    typeof value.execution === "string" &&
    record(value.request) &&
    record(value.request.params) &&
    typeof value.request.tool === "string" &&
    typeof value.request.action === "string" &&
    record(value.inputs) &&
    Object.values(value.inputs).every(
      (v) => Array.isArray(v) && v.every(savedImage),
    ) &&
    Array.isArray(value.outputs) &&
    value.outputs.every(savedImage) &&
    record(value.preservation) &&
    Array.isArray(value.notices) &&
    value.notices.every((v) => typeof v === "string")
  );
}
export function workspaceClient(api: LocalApi) {
  async function getAttempt(
    path: string,
    init?: RequestInit,
  ): Promise<Attempt> {
    const value: unknown = await (await api.request(path, init)).json();
    if (!attempt(value)) throw new Error("Saved attempt could not be read.");
    return value;
  }
  return {
    async artifact(id:string,index:number,target?:string):Promise<Json>{return (await api.request(`/api/attempts/${encodeURIComponent(id)}/artifacts/out/${index}${target?`?${new URLSearchParams({target_tool:target})}`:""}`)).json();},
    async importArtifact(id:string,index:number,target?:string):Promise<Json>{return (await api.request(`/api/attempts/${encodeURIComponent(id)}/artifacts/out/${index}/import`,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({target_tool:target})})).json();},
    preserveNative:(id:string)=>getAttempt(`/api/attempts/${encodeURIComponent(id)}/native-preservation`,{method:"POST",headers:{"Content-Type":"application/json"},body:"{}"}),
    async exampleArtifact(id:string,port:string,index:number,target:string):Promise<Json>{return (await api.request(`/api/examples/${encodeURIComponent(id)}/artifacts/${encodeURIComponent(port)}/${index}?${new URLSearchParams({target_tool:target})}`)).json();},
    async importExampleArtifact(id:string,port:string,index:number,target:string):Promise<Json>{return (await api.request(`/api/examples/${encodeURIComponent(id)}/artifacts/${encodeURIComponent(port)}/${index}/import`,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({target_tool:target})})).json();},
    async exampleNativeState(id:string,port:string,index:number):Promise<string>{const saved:unknown=await (await api.request(`/api/examples/${encodeURIComponent(id)}`)).json();if(!record(saved)||!Array.isArray(saved.input_artifacts))return "unavailable";const item=saved.input_artifacts.filter((item:unknown)=>record(item)&&item.port===port)[index];return record(item)&&typeof item.native_state==="string"?item.native_state:"workspace-dependent";},
    async snapshot(): Promise<Snapshot> {
      const value: unknown = await (await api.request("/api/state")).json();
      if (
        !record(value) ||
        !Array.isArray(value.tools) ||
        !Array.isArray(value.attempts) ||
        !value.attempts.every(summary) ||
        !value.tools.every(
          (tool: unknown) =>
            record(tool) &&
            typeof tool.id === "string" &&
            (tool.declaration === null || declaration(tool.declaration)),
        )
      )
        throw new Error("Workspace state could not be read.");
      return {
        ...value,
        tools: value.tools.map(tool => ({...tool, directory_grants: parseDirectoryGrants(tool.directory_grants)})),
      } as Snapshot;
    },
    async description(id: string): Promise<Declaration> {
      const value: unknown = await (
        await api.request(`/api/tools/${encodeURIComponent(id)}/description`)
      ).json();
      if (!declaration(value))
        throw new Error("Tool declaration is unsupported.");
      return value;
    },
    async refresh(id: string): Promise<Declaration> {
      const value: unknown = await (await api.request(`/api/tools/${encodeURIComponent(id)}/refresh`, {
        method: "POST", headers: { "Content-Type": "application/json" }, body: "{}",
      })).json();
      if (!declaration(value)) throw new Error("Tool declaration is unsupported.");
      return value;
    },
    async restart(id: string): Promise<Declaration> {
      const value: unknown = await (await api.request(`/api/tools/${encodeURIComponent(id)}/restart`, {
        method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ confirmed: true }),
      })).json();
      if (!declaration(value)) throw new Error("Tool declaration is unsupported.");
      return value;
    },
    async history(
      cursor?: number,
      tool?: string,
      action?: string,
    ): Promise<HistoryPage> {
      const query = new URLSearchParams({ limit: "20" });
      if (cursor !== undefined) query.set("cursor", String(cursor));
      if (tool) query.set("tool", tool);
      if (action) query.set("action", action);
      const value: unknown = await (
        await api.request(`/api/attempts?${query}`)
      ).json();
      if (
        !record(value) ||
        !Array.isArray(value.items) ||
        !value.items.every(summary) ||
        (value.cursor !== null && typeof value.cursor !== "number")
      )
        throw new Error("History could not be read.");
      return value as HistoryPage;
    },
    async baseline(tool: string, action: string): Promise<Attempt | null> {
      const value: unknown = await (
        await api.request(
          `/api/baselines?${new URLSearchParams({ tool, action })}`,
        )
      ).json();
      if (value === null) return null;
      if (typeof value !== "string")
        throw new Error("Baseline could not be read.");
      return getAttempt(`/api/attempts/${encodeURIComponent(value)}`);
    },
    async pin(tool: string, action: string, id: string | null): Promise<void> {
      await api.request("/api/baselines", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tool, action, attempt_id: id }),
      });
    },
    attempt: (id: string) =>
      getAttempt(`/api/attempts/${encodeURIComponent(id)}`),
    cancel: (id: string) =>
      getAttempt(`/api/attempts/${encodeURIComponent(id)}/cancel`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: "{}",
      }),
    async submitPrepared(request: Submission): Promise<Attempt> {
      const saved = await getAttempt(`/api/preparations/${encodeURIComponent(request.id)}/submit`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      });
      if (saved.id !== request.id || !sameJson(normalizedSubmission(saved.request),normalizedSubmission(request)))
        throw new Error("The acknowledgement belongs to a different submission. Retry the original submission.");
      return saved;
    },
    submit: (request: Submission) =>
      getAttempt("/api/attempts", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      }),
  };
}
