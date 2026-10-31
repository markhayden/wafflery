import {LocalApi} from "./client";
import {isAsset,type Asset} from "./assets";
import type {Declaration,Json,Schema} from "../features/actions/model";
import type {Coordinates} from "../components/comparison";
export type SavedImage={asset:Asset;image:Coordinates;role:string};
export type Submission={id:string;tool:string;action:string;revision:string;params:Schema;images:Record<string,string[]>;frames?:Record<string,{projection:"flat"|"equirectangular";topology:"bounded"|"wrap"}[]>};
export type Attempt={id:string;sequence:number;created_ms:number;request:Submission;arguments:Json;declaration:Declaration;inputs:Record<string,SavedImage[]>;wire:Json;raw_response:Json;result:Json;execution:string;preservation:{contract:string;native:string;report:string};outputs:SavedImage[];journal:Json;notices:string[]};
export type RegisteredTool={id:string;declaration:Declaration|null;error:string|null;busy:boolean;progress?:{message?:string}|null};
export type AttemptSummary={id:string;sequence:number;created_ms:number;tool:string;action:string;execution:string};
export type Snapshot={event_session?:string;tools:RegisteredTool[];attempts:AttemptSummary[]};
export type HistoryPage={items:AttemptSummary[];cursor:number|null};
function summary(value:unknown):value is AttemptSummary{return record(value)&&typeof value.id==="string"&&typeof value.sequence==="number"&&typeof value.tool==="string"&&typeof value.action==="string"&&typeof value.execution==="string";}
function record(value:unknown):value is Record<string,unknown>{return typeof value==="object"&&value!==null&&!Array.isArray(value);}
function declaration(value:unknown):value is Declaration {
 if(!record(value)||typeof value.revision!=="string"||!record(value.document)||value.document.contract!=="0.1.0"||!record(value.document.tool)||typeof value.document.tool.name!=="string"||!Array.isArray(value.document.actions))return false;
 return value.document.actions.every((action:unknown)=>record(action)&&typeof action.name==="string"&&typeof action.title==="string"&&typeof action.purpose==="string"&&record(action.params_schema)&&record(action.cost)&&typeof action.cost.may_spend==="boolean");
}
function savedImage(value:unknown):value is SavedImage {return record(value)&&isAsset(value.asset)&&record(value.image)&&typeof value.image.width==="number"&&typeof value.image.height==="number"&&typeof value.image.projection==="string"&&typeof value.image.topology==="string"&&typeof value.role==="string";}
function attempt(value:unknown):value is Attempt {
 return record(value)&&typeof value.id==="string"&&typeof value.execution==="string"&&record(value.request)&&record(value.request.params)&&typeof value.request.tool==="string"&&typeof value.request.action==="string"&&record(value.inputs)&&Object.values(value.inputs).every(v=>Array.isArray(v)&&v.every(savedImage))&&Array.isArray(value.outputs)&&value.outputs.every(savedImage)&&record(value.preservation)&&Array.isArray(value.notices)&&value.notices.every(v=>typeof v==="string");
}
export function workspaceClient(api:LocalApi) {
 async function getAttempt(path:string,init?:RequestInit):Promise<Attempt>{const value:unknown=await(await api.request(path,init)).json();if(!attempt(value))throw new Error("Saved attempt could not be read.");return value;}
 return {
  async snapshot():Promise<Snapshot>{const value:unknown=await(await api.request("/api/state")).json();if(!record(value)||!Array.isArray(value.tools)||!Array.isArray(value.attempts)||!value.attempts.every(summary)||!value.tools.every((tool:unknown)=>record(tool)&&typeof tool.id==="string"&&(tool.declaration===null||declaration(tool.declaration))))throw new Error("Workspace state could not be read.");return value as Snapshot;},
  async description(id:string):Promise<Declaration>{const value:unknown=await(await api.request(`/api/tools/${encodeURIComponent(id)}/description`)).json();if(!declaration(value))throw new Error("Tool declaration is unsupported.");return value;},
  async history(cursor?:number,tool?:string,action?:string):Promise<HistoryPage>{
   const query=new URLSearchParams({limit:"20"});if(cursor!==undefined)query.set("cursor",String(cursor));if(tool)query.set("tool",tool);if(action)query.set("action",action);
   const value:unknown=await(await api.request(`/api/attempts?${query}`)).json();
   if(!record(value)||!Array.isArray(value.items)||!value.items.every(summary)||(value.cursor!==null&&typeof value.cursor!=="number"))throw new Error("History could not be read.");return value as HistoryPage;
  },
  async baseline(tool:string,action:string):Promise<Attempt|null>{
   const value:unknown=await(await api.request(`/api/baselines?${new URLSearchParams({tool,action})}`)).json();
   if(value===null)return null;if(typeof value!=="string")throw new Error("Baseline could not be read.");return getAttempt(`/api/attempts/${encodeURIComponent(value)}`);
  },
  async pin(tool:string,action:string,id:string|null):Promise<void>{await api.request("/api/baselines",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({tool,action,attempt_id:id})});},
  attempt:(id:string)=>getAttempt(`/api/attempts/${encodeURIComponent(id)}`),
  cancel:(id:string)=>getAttempt(`/api/attempts/${encodeURIComponent(id)}/cancel`,{method:"POST",headers:{"Content-Type":"application/json"},body:"{}"}),
  submit:(request:Submission)=>getAttempt("/api/attempts",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(request)}),
 };
}
