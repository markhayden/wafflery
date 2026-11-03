import type {LocalApi} from "./client";
import type {Schema} from "../features/actions/model";
export type BundleStep={contract:string;tool:{name:string;version:string};invocation:{action:string;arguments:Schema;effective:Schema;requirements:string[]};result:Schema;inputs:{slot?:string;port?:string;image:Schema;member:string}[];artifacts:{port:string;artifact:Schema;member:string;sha256:string}[];reference_only:{slot?:string;port?:string;why:string}[];unportable:{port:string;artifact:Schema;reason:string}[]};
export type Manifest={bundle_version:1;steps:BundleStep[]};
export type ChainEdge={consumer:number;name:string;index:number;producer:{step:number;port:string;item:number;aliases:{profile:string;port:string;index:number;item:number}[]}};
export type ChainReview={edges:ChainEdge[];problem:string|null};
export type BundleRecord={chain?:ChainReview;id:string;sha256:string;size:number;manifest:Manifest;members:{name:string;sha256:string;bytes:number}[];workspace_dependent:boolean};
export type BundleSummary={id:string;sequence:number;size:number;workspace_dependent:boolean;tool:{name:string;version:string};action:string};
type Page={items:BundleSummary[];cursor:number|null};
const object=(v:unknown):v is Record<string,unknown>=>!!v&&typeof v==="object"&&!Array.isArray(v);
const hash=(v:unknown):v is string=>typeof v==="string"&&/^[a-f0-9]{64}$/.test(v);
const size=(v:unknown):v is number=>typeof v==="number"&&Number.isSafeInteger(v)&&v>=0;
const tool=(v:unknown):v is BundleStep["tool"]=>object(v)&&typeof v.name==="string"&&/^[a-z][a-z0-9-]*$/.test(v.name)&&typeof v.version==="string";
function step(v:unknown):v is BundleStep {
 if(!object(v)||v.contract!=="0.4.0"||!tool(v.tool)||!object(v.invocation)||!object(v.result))return false;
 const invocation=v.invocation;
 if(typeof invocation.action!=="string"||!object(invocation.arguments)||!object(invocation.effective)||!Array.isArray(invocation.requirements)||!invocation.requirements.every(v=>typeof v==="string"))return false;
 return Array.isArray(v.inputs)&&v.inputs.every(v=>object(v)&&typeof v.member==="string"&&object(v.image)&&(typeof v.slot==="string"||typeof v.port==="string"))&&
  Array.isArray(v.artifacts)&&v.artifacts.every(v=>object(v)&&typeof v.port==="string"&&typeof v.member==="string"&&hash(v.sha256)&&object(v.artifact))&&
  Array.isArray(v.reference_only)&&v.reference_only.every(v=>object(v)&&typeof v.why==="string"&&(typeof v.slot==="string"||typeof v.port==="string"))&&
  Array.isArray(v.unportable)&&v.unportable.every(v=>object(v)&&typeof v.port==="string"&&typeof v.reason==="string"&&object(v.artifact));
}
function chain(v:unknown,steps:number):v is ChainReview {
 return object(v)&&(v.problem===null||typeof v.problem==="string")&&Array.isArray(v.edges)&&v.edges.every(e=>object(e)&&size(e.consumer)&&e.consumer<steps&&typeof e.name==="string"&&size(e.index)&&object(e.producer)&&size(e.producer.step)&&e.producer.step<e.consumer&&typeof e.producer.port==="string"&&size(e.producer.item)&&Array.isArray(e.producer.aliases)&&e.producer.aliases.length>0&&e.producer.aliases.every(a=>object(a)&&["artifacts","images"].includes(String(a.profile))&&typeof a.port==="string"&&size(a.index)&&size(a.item)));
}
export function isBundleRecord(v:unknown):v is BundleRecord {
 if(!object(v)||!hash(v.id)||v.id!==v.sha256||!size(v.size)||v.size>256*1024*1024||!object(v.manifest)||v.manifest.bundle_version!==1||!Array.isArray(v.manifest.steps)||!v.manifest.steps.length||!v.manifest.steps.every(step)||typeof v.workspace_dependent!=="boolean"||!Array.isArray(v.members))return false;
 return (v.chain===undefined||chain(v.chain,v.manifest.steps.length))&&v.workspace_dependent===v.manifest.steps.some(s=>s.reference_only.length>0||s.unportable.length>0)&&v.members.length<=63&&v.members.every(v=>object(v)&&typeof v.name==="string"&&hash(v.sha256)&&size(v.bytes));
}
export function cliBundleTool(manifest:Manifest,review?:ChainReview):string|null {
 const names=new Set(manifest.steps.map(s=>s.tool.name));
 return (manifest.steps.length===1||!!review&&review.problem===null)&&names.size===1&&manifest.steps.length>0&&manifest.steps.every(s=>s.reference_only.length===0&&s.unportable.length===0)?manifest.steps[0]!.tool.name:null;
}
export function bundleClient(api:LocalApi){
 async function read(response:Response):Promise<BundleRecord>{const value:unknown=await response.json();if(!isBundleRecord(value))throw Error("The saved invocation bundle is unavailable.");return value;}
 return {
  async list(attempt?:string,cursor?:number):Promise<Page>{const path=attempt===undefined?"/api/bundles":`/api/attempts/${encodeURIComponent(attempt)}/bundles`;const v:unknown=await(await api.request(`${path}${cursor===undefined?"":`?cursor=${cursor}`}`)).json();if(!object(v)||!Array.isArray(v.items)||(v.cursor!==null&&!size(v.cursor))||!v.items.every(v=>object(v)&&hash(v.id)&&size(v.sequence)&&size(v.size)&&typeof v.workspace_dependent==="boolean"&&tool(v.tool)&&typeof v.action==="string"))throw Error("Saved invocation bundles could not be read.");return v as Page;},
  read:async(id:string)=>read(await api.request(`/api/bundles/${encodeURIComponent(id)}`)),
  create:async(attempt:string,omit:Record<string,string>={})=>read(await api.request(`/api/attempts/${encodeURIComponent(attempt)}/bundles`,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({omit})})),
  createChain:async(attempts:string[])=>read(await api.request("/api/bundles",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({attempts})})),
  download:async(id:string)=>(await api.request(`/api/bundles/${encodeURIComponent(id)}/download`)).blob(),
 };
}
