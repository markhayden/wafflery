import {test,expect} from "bun:test";
import {LocalApi} from "../../src/api/client";
import {bundleImports,type ImportProgress} from "../../src/api/bundle-imports";
const status:ImportProgress={phase:"completed",bundle:"a".repeat(64),step:0,tool:"sample",action:"shape",params:{amount:0.75},inputs:[{name:"token",index:0,status:"unresolved",binding:null,reason:"Choose a replacement"}],error:null};
class Reply extends LocalApi {
 path="";body:unknown;
 constructor(private value:unknown){super("http://localhost","test-only-value");}
 override async request(path:string,init?:RequestInit){this.path=path;this.body=init?.body;return Response.json(this.value);}
}
test("an explicit selected-step import sends only the receiver and step, never archived authorization",async()=>{
 const api=new Reply(status);expect(await bundleImports(api).start("b".repeat(32),0,"installed")).toEqual(status);
 expect(api.path).toBe(`/api/bundle-reviews/${"b".repeat(32)}/import`);expect(JSON.parse(String(api.body))).toEqual({step:0,tool:"installed"});
});
test("missing progress stays missing and invalid progress cannot become completed",async()=>{
 expect(await bundleImports(new Reply(null)).progress("review")).toBeNull();
 for(const value of [{...status,phase:"ready_to_run"},{...status,step:-1},{...status,inputs:[{...status.inputs[0],status:"trusted"}]},{...status,inputs:[{...status.inputs[0],index:-1}]}])await expect(bundleImports(new Reply(value)).progress("review")).rejects.toThrow("progress is unavailable");
});
test("upload review rejects a malformed or unsupported manifest",async()=>{
 await expect(bundleImports(new Reply({id:"b".repeat(32),record:{manifest:{bundle_version:2}},import:null})).read("review")).rejects.toThrow("review is unavailable");
 await expect(bundleImports(new Reply({items:new Array(3).fill({id:"b".repeat(32),bundle:"a".repeat(64),phase:"review"})})).list()).rejects.toThrow("could not be read");
});
