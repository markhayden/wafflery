import { expect, test } from "bun:test";
import { LocalApi } from "../../src/api/client";
import { workspaceClient, type Submission } from "../../src/api/workspace";
const request: Submission = {id:"paid/draft",tool:"sample",action:"price_test",revision:"r1",
  params:{max_spend_usd:0.5,label:"reviewed"},images:{references:["first","second"]},frames:{}};
function response() { return {id:request.id,execution:"submitted",request:structuredClone(request),
  inputs:{},outputs:[],preservation:{contract:"pending"},notices:[]}; }
class Reply extends LocalApi {
  calls: {path:string;init:RequestInit}[] = [];
  constructor(public value: unknown) { super("http://127.0.0.1:1234","fixture"); }
  override async request(path:string,init:RequestInit = {}) {
    this.calls.push({path,init}); return Response.json(this.value);
  }
}
test("paid acknowledgement matches the exact request with unordered object keys and ordered image lists",async () => {
  const value = response();
  value.request.params = {label:"reviewed",max_spend_usd:0.5}; delete value.request.frames;
  const api = new Reply(value);
  await expect(workspaceClient(api).submitPrepared(request)).resolves.toMatchObject(value);
  expect(api.calls).toEqual([{path:"/api/preparations/paid%2Fdraft/submit",init:{method:"POST",
    headers:{"Content-Type":"application/json"},body:JSON.stringify(request)}}]);
});
test("wrong paid acknowledgements fail without retrying or accepting another attempt",async () => {
  for (const change of [
    (value:ReturnType<typeof response>) => { value.id = "other"; },
    (value:ReturnType<typeof response>) => { value.request.id = "other"; },
    (value:ReturnType<typeof response>) => { value.request.params.max_spend_usd = 2; },
    (value:ReturnType<typeof response>) => { value.request.images.references!.reverse(); },
    (value:ReturnType<typeof response>) => { value.request.frames = {references:[{projection:"flat",topology:"bounded"}]}; },
  ]) {
    const value = response(); change(value);
    const api = new Reply(value);
    await expect(workspaceClient(api).submitPrepared(request)).rejects.toThrow("different submission");
    expect(api.calls).toHaveLength(1);
  }
});
