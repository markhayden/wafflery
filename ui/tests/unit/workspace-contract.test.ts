import { expect, test } from "bun:test";
import { LocalApi } from "../../src/api/client";
import { workspaceClient } from "../../src/api/workspace";
import type { Declaration } from "../../src/features/actions/model";
const declaration: Declaration = {
  revision: "current", generation: 0, schemas: {}, document: { contract: "0.3.0", tool: { name: "arbitrary", version: "1", purpose: "Boundary fixture" },
    actions: [{name:"check",title:"Check",purpose:"Test",params_schema:{},cost:{may_spend:false},readiness:{state:"ready",requirements:[]}}] }
};
class ResponseApi extends LocalApi {
  constructor(private value: unknown) { super("http://127.0.0.1:1234", "fixture"); }
  override async request(): Promise<Response> { return Response.json(this.value); }
}
test("live declarations require v0.3 readiness without accepting older versions", async () => {
  await expect(workspaceClient(new ResponseApi(declaration)).description("arbitrary")).resolves.toEqual(declaration);
  for (const version of ["0.1.0", "0.2.0"]) {
    const old = structuredClone(declaration); old.document.contract = version;
    await expect(workspaceClient(new ResponseApi(old)).description("arbitrary")).rejects.toThrow("unsupported");
  }
  const missing = JSON.parse(JSON.stringify(declaration));
  delete missing.document.actions[0].readiness;
  await expect(workspaceClient(new ResponseApi(missing)).description("arbitrary")).rejects.toThrow("unsupported");
});

test("malformed readiness does not cross the browser boundary", async () => {
  for (const readiness of [
    { state: ["ready"], requirements: [] },
    { state: "ready", requirements: [{ kind: "credential", name: "KEY", state: "maybe" }] },
    { state: "ready", requirements: [{ kind: ["env"], name: "REGION", state: "present" }] },
    { state: "ready", requirements: [{ kind: "env", name: "REGION", state: "present", setup: 42 }] },
  ]) {
    const malformed = JSON.parse(JSON.stringify(declaration));
    malformed.document.actions[0].readiness = readiness;
    await expect(workspaceClient(new ResponseApi(malformed)).description("arbitrary")).rejects.toThrow("unsupported");
  }
});
