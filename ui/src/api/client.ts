export type ControlRefusal={reason:"invalid_arguments";run:null;error:{code:"usage"|"input";retryable:false;reason:string}};
function controlRefusal(error:object):ControlRefusal|undefined{
 if(!("code" in error)||error.code!=="artifact_refusal"||!("refusal" in error)||!error.refusal||typeof error.refusal!=="object")return;
 const value=error.refusal as Record<string,unknown>,detail=value.error;
 if(value.reason!=="invalid_arguments"||value.run!==null||!detail||typeof detail!=="object")return;
 const body=detail as Record<string,unknown>;
 if((body.code==="input"||body.code==="usage")&&body.retryable===false&&typeof body.reason==="string")return value as ControlRefusal;
}
export type BindingIssue = {port:string; index?:number; problem:"wrong_type"|"wrong_version"|"wrong_frame"|"unresolved"; expected:string; got:string; remedy?:string};
export function bindingIssues(error: object): BindingIssue[] {
  if (!("code" in error) || error.code!=="binding" || !("bindings" in error) || !Array.isArray(error.bindings)) return [];
  const values: unknown[]=error.bindings;
  return values.every((item):item is BindingIssue=>{
    if (!item || typeof item!=="object") return false;
    const value=item as Record<string,unknown>;
    return typeof value.port==="string" && typeof value.expected==="string" && typeof value.got==="string" &&
      typeof value.problem==="string" && ["wrong_type","wrong_version","wrong_frame","unresolved"].includes(value.problem) &&
      (value.index===undefined || (typeof value.index==="number" && Number.isSafeInteger(value.index) && value.index>=0)) &&
      (value.remedy===undefined || typeof value.remedy==="string") &&
      (value.problem!=="unresolved" || typeof value.remedy==="string");
  }) ? values : [];
}
export class ApiError extends Error {
  constructor(message: string, readonly status: number, readonly bindings: BindingIssue[] = [], readonly refusal?:ControlRefusal) { super(message); }
}

/** Only the host origin receives credentials. Tool viewers use separate, uncredentialed URLs. */
export class LocalApi {
  constructor(private readonly origin: string, private readonly token: string) {}

  async request(path: string, init: RequestInit = {}): Promise<Response> {
    const url = new URL(path, this.origin);
    if (!path.startsWith("/api/") || path.includes("\\") || url.origin !== this.origin || !url.pathname.startsWith("/api/")) {
      throw new Error("Only local workspace API paths are allowed.");
    }
    const headers = new Headers(init.headers);
    headers.set("Authorization", `Bearer ${this.token}`);
    const response = await fetch(url, { ...init, headers, credentials: "omit", mode: "same-origin", redirect: "error" });
    if (response.status === 401) throw new ApiError("Your workspace session ended. Run wafflery to reopen it.", response.status);
    if (!response.ok) {
      const payload: unknown = await response.json().catch(() => null);
      if (payload && typeof payload === "object" && "error" in payload && payload.error && typeof payload.error === "object" && "message" in payload.error && typeof payload.error.message === "string") {
        throw new ApiError(payload.error.message, response.status, bindingIssues(payload.error),controlRefusal(payload.error));
      }
      throw new ApiError("The workspace could not complete that request.", response.status);
    }
    return response;
  }
}
