import type {LocalApi} from "./client";
import {isBundleRecord,type BundleRecord} from "./bundles";
import type {InputBinding,BindingOrigin,ReplayContext} from "./workspace";
import type {Schema} from "../features/actions/model";
export type ImportProgress={replay?:ReplayContext|null;phase:"importing"|"completed"|"failed";bundle:string;step:number;tool:string;action:string;params:Schema;inputs:{name:string;index:number;status:"pending"|"importing"|"completed"|"unresolved"|"failed";binding:InputBinding|null;reason:string|null;source?:BindingOrigin|null;changed?:boolean}[];error:string|null};
export type BundleUpload={id:string;record:BundleRecord;import:ImportProgress|null};
export type ReviewSummary={id:string;bundle:string;phase:string};
const object=(v:unknown):v is Record<string,unknown>=>!!v&&typeof v==="object"&&!Array.isArray(v);
const hash=(v:unknown)=>typeof v==="string"&&/^[a-f0-9]{64}$/.test(v);
const id=(v:unknown)=>typeof v==="string"&&/^[a-f0-9]{32}$/.test(v);
export function isReplayContext(v:unknown):v is ReplayContext{return object(v)&&id(v.id)&&typeof v.generation==="number"&&Number.isSafeInteger(v.generation)&&v.generation>0;}
export function isBindingOrigin(v:unknown):v is BindingOrigin{return object(v)&&(typeof v.attempt==="string"||typeof v.example==="string")&&typeof v.port==="string"&&typeof v.occurrence==="number"&&Number.isSafeInteger(v.occurrence)&&v.occurrence>=0;}
export function isImportProgress(v:unknown):v is ImportProgress {
 return object(v)&&(v.replay===undefined||v.replay===null||isReplayContext(v.replay))&&["importing","completed","failed"].includes(String(v.phase))&&hash(v.bundle)&&typeof v.step==="number"&&Number.isSafeInteger(v.step)&&v.step>=0&&typeof v.tool==="string"&&typeof v.action==="string"&&object(v.params)&&(v.error===null||typeof v.error==="string")&&Array.isArray(v.inputs)&&v.inputs.every(i=>object(i)&&typeof i.name==="string"&&typeof i.index==="number"&&Number.isSafeInteger(i.index)&&i.index>=0&&["pending","importing","completed","unresolved","failed"].includes(String(i.status))&&(i.reason===null||typeof i.reason==="string")&&(i.binding===null||object(i.binding))&&(i.source===undefined||i.source===null||isBindingOrigin(i.source))&&(i.changed===undefined||typeof i.changed==="boolean"));
}
export function bundleImports(api:LocalApi){
 async function review(response:Response):Promise<BundleUpload>{const v:unknown=await response.json();if(!object(v)||!id(v.id)||!isBundleRecord(v.record)||(v.import!==null&&!isImportProgress(v.import)))throw Error("The uploaded bundle review is unavailable.");return v as BundleUpload;}
 return {
  async list():Promise<ReviewSummary[]>{const v:unknown=await(await api.request("/api/bundle-reviews")).json();if(!object(v)||!Array.isArray(v.items)||v.items.length>2||!v.items.every(i=>object(i)&&id(i.id)&&hash(i.bundle)&&typeof i.phase==="string"))throw Error("Bundle uploads could not be read.");return v.items as ReviewSummary[];},
  upload:(file:File)=>reviewUpload(file),
  read:(value:string)=>reviewResponse(value),
  async remove(value:string){await api.request(`/api/bundle-reviews/${encodeURIComponent(value)}`,{method:"DELETE"});},
  async start(value:string,step:number,tool:string):Promise<ImportProgress>{const v:unknown=await(await api.request(`/api/bundle-reviews/${encodeURIComponent(value)}/import`,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({step,tool})})).json();if(!isImportProgress(v))throw Error("Import progress is unavailable. Reopen this review before retrying.");return v;},
  async progress(value:string):Promise<ImportProgress|null>{const v:unknown=await(await api.request(`/api/bundle-reviews/${encodeURIComponent(value)}/progress`)).json();if(v!==null&&!isImportProgress(v))throw Error("Import progress is unavailable. Reopen this review before retrying.");return v;},
 };
 async function reviewUpload(file:File){if(file.size>256*1024*1024)throw Error("Invocation bundle exceeds 256 MiB.");return review(await api.request("/api/bundle-reviews",{method:"POST",headers:{"Content-Type":"application/x-tar"},body:file}));}
 async function reviewResponse(value:string){return review(await api.request(`/api/bundle-reviews/${encodeURIComponent(value)}`));}
}
