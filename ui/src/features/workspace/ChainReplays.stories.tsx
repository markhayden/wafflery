import type {Meta,StoryObj} from "@storybook/react-vite";
import {LocalApi} from "../../api/client";
import type {ReplayStatus} from "../../api/chain-replays";
import type {BundleRecord} from "../../api/bundles";
import {Drawer} from "../../components/Drawer";
import {ChainReplays} from "./ChainReplays";
const id="a".repeat(32),hash="b".repeat(64);
const step={contract:"0.6.0",tool:{name:"rascal",version:"0.1.0"},invocation:{action:"rasterize",arguments:{width:16},effective:{},requirements:[]},result:{},inputs:[],artifacts:[],reference_only:[],unportable:[]};
const record:BundleRecord={id:hash,sha256:hash,size:4096,manifest:{bundle_version:1,steps:[step,{...step,invocation:{...step.invocation,action:"feather"}}]},members:[],workspace_dependent:false,chain:{edges:[],problem:null}};
function status(phase:ReplayStatus["phase"]):ReplayStatus{return {settling:phase==="running"||phase==="importing",id,bundle:hash,tools:["installed","installed"],step:phase==="completed"?2:1,total:2,generation:3,phase,attempts:[{step:0,attempt:"fresh-producer",outcome:"completed"}],error:phase==="blocked"?"The required producer output is unavailable. No consumer was prepared.":null,import:["awaiting_run","importing","blocked"].includes(phase)?{replay:{id,generation:3},bundle:hash,step:1,tool:"installed",action:"feather",params:{radius:3},phase:phase==="importing"?"importing":"completed",inputs:[{name:"mask",index:0,status:phase==="importing"?"importing":"completed",binding:{kind:"artifact",reference:{type:"rascal:Mask",schema_version:1,tool:"rascal",identity:`sha256:${"c".repeat(64)}`,frame:"opaque-grid",resource:`rascal://artifact/sha256:${"c".repeat(64)}`}},reason:null,source:{attempt:"fresh-producer",port:"mask",occurrence:0},changed:true}],error:null}:null};}
class Preview extends LocalApi{
 private value:ReplayStatus;constructor(private mode:ReplayStatus["phase"]|"loading"|"failed"|"empty"|"settling"){super("http://localhost","synthetic-only");this.value=status(mode==="settling"?"stopped":["loading","failed","empty"].includes(mode)?"ready":mode as ReplayStatus["phase"]);if(mode==="settling")this.value.settling=true;}
 override async request(path:string,init?:RequestInit):Promise<Response>{
  if(this.mode==="loading")return new Promise(()=>{});if(this.mode==="failed")throw Error("Chain review could not be read. Reopen it before continuing.");
  if(path.startsWith("/api/bundles/"))return Response.json(record);
  if(init?.method==="POST"){if(path.endsWith("/stage"))this.value=status("awaiting_run");if(path.endsWith("/stop"))this.value=status("stopped");return Response.json(this.value);}
  if(init?.method==="DELETE"){this.mode="empty";return new Response(null,{status:204});}
  return Response.json(path==="/api/chain-replays"?(this.mode==="empty"?[]:[this.value]):this.value);
 }
}
const meta={title:"Workspace/ChainReplays",component:ChainReplays,args:{api:new Preview("awaiting_run"),onOpen:async()=>{}},render:args=><Drawer open title="Chain reviews" onClose={()=>{}}><ChainReplays {...args}/></Drawer>} satisfies Meta<typeof ChainReplays>;
export default meta;type Story=StoryObj<typeof meta>;
export const FreshOutput:Story={};
export const Ready:Story={args:{api:new Preview("ready")}};
export const Importing:Story={args:{api:new Preview("importing")}};
export const Running:Story={args:{api:new Preview("running")}};
export const Blocked:Story={args:{api:new Preview("blocked")}};
export const Stopped:Story={args:{api:new Preview("stopped")}};
export const Settling:Story={args:{api:new Preview("settling")}};
export const Completed:Story={args:{api:new Preview("completed")}};
export const Loading:Story={args:{api:new Preview("loading")}};
export const Failed:Story={args:{api:new Preview("failed")}};
export const Empty:Story={args:{api:new Preview("empty")}};
