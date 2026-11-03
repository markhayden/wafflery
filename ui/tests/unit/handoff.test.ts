import {test,expect} from "bun:test";
import {LocalApi} from "../../src/api/client";
import {handoffClient,type Handoff} from "../../src/api/handoff";
const local:Handoff={state:"local",blockers:[],invocation:{action:"shape"},arguments:{amount:0.75},arguments_filename:"wafflery-arguments.json",argv:["study","call","shape"],command:"'study' 'call' 'shape'",cwd:"/tmp/work",allowed_dirs:["/tmp/work"],requirements:["PROVIDER_KEY"]};
class Reply extends LocalApi{paths:string[]=[];constructor(private value:unknown){super("http://localhost","test-only-token");}override async request(path:string){this.paths.push(path);return Response.json(this.value);}}
test("a contradictory unavailable handoff can never expose a copyable command",async()=>{
 for(const value of [{...local,state:"unavailable"},{...local,blockers:["Missing owned input"]},{...local,invocation:null}])await expect(handoffClient(new Reply(value)).read("saved")).rejects.toThrow("unavailable");
});
test("handoff reads are scoped to an encoded saved id and retain resolved values",async()=>{
 const api=new Reply(local);const saved=await handoffClient(api).read("saved/with spaces");
 expect(saved.arguments?.amount).toBe(0.75);expect(api.paths).toEqual(["/api/attempts/saved%2Fwith%20spaces/handoff"]);
});
