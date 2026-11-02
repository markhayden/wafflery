import { expect, test } from "bun:test";
import { LocalApi } from "../../src/api/client";
import { estimateClient, parseEstimate, positiveCap, estimateDeadline, type PreparedEstimate } from "../../src/api/estimates";
import type { Submission } from "../../src/api/workspace";

const preparation = { id: "draft", tool: "registered", action: "price", revision: "revision", expires_ms: 20000, bytes: 123 };
const session = {
  contract: "0.3.0" as const, tool: { name: "peer", version: "1" },
  id: "22222222-2222-4222-8222-222222222222", started: "2026-11-01T00:00:00Z",
  budget_usd: 2, spent_usd: 0, reserved_usd: 0, unresolved_usd: 0, remaining_usd: 2,
};
function quote(): PreparedEstimate {
  return { preparation: {...preparation}, estimate: {
    effective: { model: "scripted", config_identity: `sha256:${"a".repeat(64)}` },
    worst_case_usd: 0.1, cap_usd: 0.5, allowed: true, refused: null,
    readiness: { state: "ready", requirements: [] }, session: {...session},
    expires: "1970-01-01T00:00:10Z",
  }};
}
const request: Submission = {
  id: "draft", tool: "registered", action: "price", revision: "revision",
  params: { label: "typed", max_spend_usd: 0.5 }, images: {references: ["first", "second"]},
};
class ResponseApi extends LocalApi {
  calls: {path:string; init:RequestInit}[] = [];
  constructor(public value: unknown) { super("http://127.0.0.1:1234", "fixture"); }
  override async request(path: string, init: RequestInit = {}): Promise<Response> {
    this.calls.push({path, init});
    return init.method === "DELETE" ? new Response(null, {status:204}) : Response.json(this.value);
  }
}
test("caps accept explicit positive decimals without clamping or inventing defaults", () => {
  for (const value of [undefined, null, false, {}, [], "", " ", "0", "-1", "Infinity", "NaN", "1e999", "0x10", 0, -1, Infinity, NaN])
    expect(positiveCap(value)).toBeUndefined();
  for (const [value, expected] of [[".5",0.5], ["1.",1], ["1e-6",0.000001], [" 2.50 ",2.5], [1000,1000]] as const)
    expect(positiveCap(value)).toBe(expected);
});
test("estimate display preserves unknown prices and null session limits", () => {
  const value: any = quote();
  value.estimate.allowed = false; value.estimate.refused = "readiness";
  value.estimate.worst_case_usd = null; value.estimate.cap_usd = null;
  value.estimate.effective = {model:null, config_identity:null};
  value.estimate.readiness = {state:"needs_setup",requirements:[{kind:"env",name:"REGION",state:"missing",setup:"Set REGION."}]};
  value.estimate.session.budget_usd = null; value.estimate.session.remaining_usd = null;
  const parsed = parseEstimate(value);
  expect(parsed.estimate.worst_case_usd).toBeNull();
  expect(parsed.estimate.cap_usd).toBeNull();
  expect(parsed.estimate.session.remaining_usd).toBeNull();
  expect(parsed.estimate.readiness.requirements[0]?.setup).toBe("Set REGION.");
  expect(estimateDeadline(parsed)).toBe(10000);
  parsed.preparation.expires_ms = 5000;
  expect(estimateDeadline(parsed)).toBe(5000);
});
test("malformed or contradictory estimate displays fail closed", () => {
  const mutations: ((value:any)=>void)[] = [
    value => value.preparation.bytes = -1,
    value => value.preparation.expires_ms = Infinity,
    value => value.estimate.expires = "unknown",
    value => value.estimate.worst_case_usd = "0.1",
    value => value.estimate.cap_usd = 0,
    value => value.estimate.effective.model = {},
    value => value.estimate.effective.config_identity = "not a digest",
    value => value.estimate.session.remaining_usd = 8,
    value => value.estimate.readiness.requirements = [{kind:"env",name:"REGION",state:"maybe"}],
    value => value.estimate.refused = "cap",
    value => value.estimate.worst_case_usd = null,
    value => value.estimate.readiness.state = "needs_setup",
  ];
  for (const mutate of mutations) {
    const value = quote(); mutate(value);
    expect(() => parseEstimate(value)).toThrow();
  }
});
test("browser controls send exact inputs once and never caller quote fields", async () => {
  const api = new ResponseApi(preparation), client = estimateClient(api);
  await expect(client.prepare(request)).resolves.toEqual(preparation);
  expect(api.calls[0]).toEqual({path:"/api/preparations",init:{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(request)}});
  api.value = quote();
  await expect(client.estimate(preparation, session.id)).resolves.toEqual(quote());
  expect(api.calls[1]).toEqual({path:"/api/preparations/draft/estimate",init:{method:"POST",headers:{"Content-Type":"application/json"},body:"{}"}});
  await client.read(preparation, session.id);
  expect(api.calls[2]).toEqual({path:"/api/preparations/draft/estimate",init:{}});
  await client.discard("a/b");
  expect(api.calls[3]).toEqual({path:"/api/preparations/a%2Fb",init:{method:"DELETE"}});
  expect(api.calls).toHaveLength(4);
});
test("wrong preparation or session responses are rejected without retrying", async () => {
  const api = new ResponseApi({...preparation, action:"other"}), client = estimateClient(api);
  await expect(client.prepare(request)).rejects.toThrow();
  for (const field of ["id", "tool", "action", "revision", "expires_ms", "bytes"] as const) {
    const value:any = quote(); value.preparation[field] = typeof value.preparation[field] === "number" ? 1 : "other";
    api.value = value;
    await expect(client.estimate(preparation, session.id)).rejects.toThrow();
  }
  api.value = quote();
  await expect(client.estimate(preparation, "33333333-3333-4333-8333-333333333333")).rejects.toThrow();
  expect(api.calls).toHaveLength(8);
});
