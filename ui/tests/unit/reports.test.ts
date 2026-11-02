import { expect, test } from "bun:test";
import { LocalApi } from "../../src/api/client";
import { reportClient, parseReport, type ReportView } from "../../src/api/reports";
const report: ReportView = { id: "a".repeat(32), attempt: "saved", name: "document", kind: "document", phase: "ready",
  exit_code: 0, error: null, idle_expired: false, diagnostics: "", diagnostics_truncated: false };
class ResponseApi extends LocalApi {
  constructor(public value: unknown) { super("http://127.0.0.1:1234", "fixture"); }
  override async request(): Promise<Response> { return Response.json(this.value); }
}
test("report observations and frame URLs fail closed on malformed or unrelated values", async () => {
  expect(parseReport(report)).toEqual(report);
  for (const delta of [{id:"/outside"}, {phase:"other"}, {kind:"html"}, {kind:["server"]}, {phase:["ready"]}, {error:{}}, {diagnostics:[]}, {exit_code:1.5}, {idle_expired:0}]) {
    expect(() => parseReport({...report,...delta})).toThrow();
  }
  const api = new ResponseApi(report), client = reportClient(api);
  await expect(client.open("other", "document")).rejects.toThrow();
  await expect(client.read("b".repeat(32))).rejects.toThrow();
  for (const url of ["https://example.invalid", "/api/state", "//127.0.0.1:1234/views/abc", `/views/${"c".repeat(64)}?extra`]) {
    api.value = {url}; await expect(client.frame(report.id)).rejects.toThrow();
  }
  api.value = {url:`/views/${"c".repeat(64)}`};
  await expect(client.frame(report.id)).resolves.toBe(`/views/${"c".repeat(64)}`);
  api.value = {view:null}; await expect(client.current("saved")).resolves.toBeNull();
  api.value = {view:report}; await expect(client.current("other")).rejects.toThrow();
});
