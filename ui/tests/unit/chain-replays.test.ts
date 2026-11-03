import {test,expect} from "bun:test";
import {LocalApi} from "../../src/api/client";
import {chainReplays} from "../../src/api/chain-replays";
const id="a".repeat(32),bundle="b".repeat(64);
const ready={settling:false,id,bundle,tools:["installed","installed"],step:0,total:2,generation:0,phase:"ready",attempts:[],import:null,error:null};
class Reply extends LocalApi {lastPath="";body:unknown;constructor(private value:unknown){super("http://localhost","synthetic-only");}override async request(path:string,init?:RequestInit){this.lastPath=path;this.body=init?.body;return Response.json(this.value);}}
test("chain controls submit retained identities and explicit resume without run data",async()=>{
 const api=new Reply(ready);await chainReplays(api).start(bundle,["installed","installed"]);expect(api.lastPath).toBe("/api/chain-replays");expect(JSON.parse(String(api.body))).toEqual({bundle,tools:["installed","installed"]});await chainReplays(api).stage(id,true);expect(api.lastPath).toBe(`/api/chain-replays/${id}/stage`);expect(JSON.parse(String(api.body))).toEqual({resume:true});await chainReplays(api).stop(id);expect(JSON.parse(String(api.body))).toEqual({});
});
test("invalid totals, unknown phases and contradictory ready inputs do not open chain review",async()=>{
 for(const value of [{...ready,total:17},{...ready,step:3},{...ready,phase:"auto_running"},{...ready,phase:"awaiting_run"}])await expect(chainReplays(new Reply(value)).read(id)).rejects.toThrow("unavailable");
});
