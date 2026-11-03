import { expect, test } from "bun:test";
import { LocalApi, ApiError, type BindingIssue } from "../../src/api/client";

test("bearer credentials cannot be sent to a viewer or a path outside the host API", async () => {
  const api = new LocalApi("http://127.0.0.1:1234", "test-session");
  for (const path of ["http://127.0.0.1:5678/api/state", "//example.com/api/state", "/api/../viewer", "/api/\\example.com"]) {
    await expect(api.request(path)).rejects.toThrow("Only local workspace API paths");
  }
});

test("validated indexed binding errors reach the form without flattening remedies", async()=>{
  const original=globalThis.fetch;
  const bindings: BindingIssue[]=[{port:"masks",index:2,problem:"unresolved",expected:"Mask",got:"sha256:missing",remedy:"Import the bundle."}];
  globalThis.fetch=(async()=>Response.json({error:{code:"binding",message:"Inputs could not bind",bindings}},{status:422})) as unknown as typeof fetch;
  try {
    const api=new LocalApi("http://127.0.0.1:1234","fixture");
    try {await api.request("/api/preparations/test/estimate");throw Error("Expected refusal");}
    catch(error) {expect(error).toBeInstanceOf(ApiError);expect((error as ApiError).bindings).toEqual(bindings);}
  } finally {globalThis.fetch=original;}
});

test("malformed binding problems are not coerced into valid issues", async()=>{
  const original=globalThis.fetch;
  globalThis.fetch=(async()=>Response.json({error:{code:"binding",message:"Refused",bindings:[{port:"masks",problem:["wrong_type"],expected:"Mask",got:"Token"}]}},{status:422})) as unknown as typeof fetch;
  try {await new LocalApi("http://127.0.0.1:1234","fixture").request("/api/test");throw Error("Expected refusal");}
  catch(error) {expect((error as ApiError).bindings).toEqual([]);}
  finally {globalThis.fetch=original;}
});
