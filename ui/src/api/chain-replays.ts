import type {LocalApi} from "./client";
import {isImportProgress,type ImportProgress} from "./bundle-imports";
export type ReplayStatus={settling:boolean;id:string;bundle:string;tools:string[];step:number;total:number;generation:number;phase:"ready"|"preparing"|"importing"|"awaiting_run"|"running"|"blocked"|"stopped"|"completed";attempts:{step:number;attempt:string;outcome:string}[];import:ImportProgress|null;error:string|null};
const object=(v:unknown):v is Record<string,unknown>=>!!v&&typeof v==="object"&&!Array.isArray(v);
const integer=(v:unknown):v is number=>typeof v==="number"&&Number.isSafeInteger(v)&&v>=0;
function valid(v:unknown):v is ReplayStatus{
 if(!object(v))return false;const total=v.total;
 if(typeof v.settling!=="boolean"||typeof v.id!=="string"||!/^[a-f0-9]{32}$/.test(v.id)||typeof v.bundle!=="string"||!/^[a-f0-9]{64}$/.test(v.bundle)||!integer(total)||total<1||total>16||!integer(v.step)||v.step>total||!integer(v.generation)||!Array.isArray(v.tools)||v.tools.length!==total||!v.tools.every(t=>typeof t==="string"&&t.length>0)||!["ready","preparing","importing","awaiting_run","running","blocked","stopped","completed"].includes(String(v.phase))||(v.error!==null&&typeof v.error!=="string")||!Array.isArray(v.attempts)||!v.attempts.every(a=>object(a)&&integer(a.step)&&a.step<total&&typeof a.attempt==="string"&&typeof a.outcome==="string")||(v.import!==null&&!isImportProgress(v.import)))return false;
 if(v.phase==="awaiting_run"){const p=v.import as ImportProgress|null;return !!p&&p.phase==="completed"&&p.bundle===v.bundle&&p.step===v.step&&p.tool===v.tools[v.step]&&p.replay?.id===v.id&&p.replay.generation===v.generation;}
 return v.phase!=="completed"||v.step===v.total;
}
export function chainReplays(api:LocalApi){
 async function readResponse(response:Response):Promise<ReplayStatus>{const v:unknown=await response.json();if(!valid(v))throw Error("Chain review is unavailable. Reopen it before continuing.");return v;}
 const path=(id:string)=>`/api/chain-replays/${encodeURIComponent(id)}`;
 const post=(url:string,body:unknown)=>api.request(url,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(body)}).then(readResponse);
 return {
  async list():Promise<ReplayStatus[]>{const v:unknown=await(await api.request("/api/chain-replays")).json();if(!Array.isArray(v)||v.length>2||!v.every(valid))throw Error("Chain reviews are unavailable.");return v;},
  read:(id:string)=>api.request(path(id)).then(readResponse),
  start:(bundle:string,tools:string[])=>post("/api/chain-replays",{bundle,tools}),
  startUpload:(id:string,tools:string[])=>post(`/api/bundle-reviews/${encodeURIComponent(id)}/replay`,{tools}),
  stage:(id:string,resume=false)=>post(`${path(id)}/stage`,{resume}),
  stop:(id:string)=>post(`${path(id)}/stop`,{}),
  async remove(id:string){await api.request(path(id),{method:"DELETE"});},
 };
}
