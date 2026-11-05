import {test,expect} from "bun:test";
import {LocalApi} from "../../src/api/client";
import {bundleClient,cliBundleTool} from "../../src/api/bundles";
const step={contract:"0.5.0",tool:{name:"sample",version:"1.0.0"},invocation:{action:"shape",arguments:{amount:0.75},effective:{model:null,config_identity:`sha256:${"b".repeat(64)}`},requirements:["PROVIDER_KEY"]},result:{},inputs:[],artifacts:[],reference_only:[],unportable:[]};
const saved={id:"a".repeat(64),sha256:"a".repeat(64),size:2048,manifest:{bundle_version:1 as const,steps:[step]},members:[],workspace_dependent:false};
class Reply extends LocalApi{paths:string[]=[];body:unknown;constructor(private value:unknown){super("http://localhost","test-only-token");}override async request(path:string,init?:RequestInit){this.paths.push(path);this.body=init?.body;return Response.json(this.value);}}
test("bundle review refuses a contradictory portable claim and unknown manifest version",async()=>{
 for(const value of [{...saved,manifest:{bundle_version:2,steps:[step]}},{...saved,manifest:{bundle_version:1,steps:[{...step,unportable:[{port:"token",artifact:{},reason:"No owned export"}]}]}}])await expect(bundleClient(new Reply(value)).read(saved.id)).rejects.toThrow("unavailable");
});
test("bundle creation uses only a saved id and explicit omission reasons",async()=>{
 const api=new Reply(saved);const value=await bundleClient(api).create("saved/with spaces",{source:"Left out explicitly."});
 expect(api.paths).toEqual(["/api/attempts/saved%2Fwith%20spaces/bundles"]);expect(JSON.parse(String(api.body))).toEqual({omit:{source:"Left out explicitly."}});expect(value.manifest.steps[0]?.invocation.arguments.amount).toBe(0.75);
});
test("a CLI bundle claim requires one tool and no omitted or unportable inputs",()=>{
 expect(cliBundleTool(saved.manifest)).toBe("sample");
 expect(cliBundleTool({bundle_version:1,steps:[{...step,reference_only:[{slot:"source",why:"Omitted"}]}]})).toBeNull();
 expect(cliBundleTool({bundle_version:1,steps:[step,{...step,tool:{name:"other",version:"1.0.0"}}]})).toBeNull();
});
test("selected chain creation submits only ordered run ids and validates derived edges",async()=>{
 const chain={edges:[{consumer:1,name:"token",index:0,producer:{step:0,port:"token",item:0,aliases:[{profile:"artifacts",port:"token",index:0,item:0}]}}],problem:null};
 const record={...saved,manifest:{bundle_version:1 as const,steps:[step,step]},chain};
 const api=new Reply(record);await bundleClient(api).createChain(["first","second"]);
 expect(api.paths).toEqual(["/api/bundles"]);expect(JSON.parse(String(api.body))).toEqual({attempts:["first","second"]});
 expect(cliBundleTool(record.manifest)).toBeNull();expect(cliBundleTool(record.manifest,chain)).toBe("sample");
 expect(cliBundleTool(record.manifest,{edges:[],problem:"Ambiguous output"})).toBeNull();
 for(const bad of [{...chain,edges:[{...chain.edges[0],consumer:0}]},{...chain,edges:[{...chain.edges[0],index:-1}]}])await expect(bundleClient(new Reply({...record,chain:bad})).read(saved.id)).rejects.toThrow("unavailable");
});
