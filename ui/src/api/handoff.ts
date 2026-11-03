import type {LocalApi} from "./client";
import type {Schema} from "../features/actions/model";
export type Handoff={state:"local"|"unavailable";blockers:string[];invocation:Schema|null;arguments:Schema|null;arguments_filename:string;argv:string[]|null;command:string|null;cwd:string|null;allowed_dirs:string[];requirements:string[]};
const record=(value:unknown):value is Record<string,unknown>=>!!value&&typeof value==="object"&&!Array.isArray(value);
const strings=(value:unknown):value is string[]=>Array.isArray(value)&&value.every(item=>typeof item==="string");
function valid(value:unknown):value is Handoff{return record(value)&&(value.state==="local"||value.state==="unavailable")&&strings(value.blockers)&&(value.invocation===null||record(value.invocation))&&(value.arguments===null||record(value.arguments))&&value.arguments_filename==="wafflery-arguments.json"&&(value.argv===null||strings(value.argv))&&(value.command===null||typeof value.command==="string")&&(value.cwd===null||typeof value.cwd==="string")&&strings(value.allowed_dirs)&&strings(value.requirements)&&(value.state==="local"?(value.blockers.length===0&&record(value.invocation)&&typeof value.command==="string"&&value.command.length>0&&record(value.arguments)&&strings(value.argv)&&value.argv.length>0&&typeof value.cwd==="string"):value.command===null&&value.argv===null);}
export function handoffClient(api:LocalApi){return{
 async read(id:string):Promise<Handoff>{const value:unknown=await(await api.request(`/api/attempts/${encodeURIComponent(id)}/handoff`)).json();if(!valid(value))throw Error("The saved CLI handoff is unavailable.");return value;},
 async download(id:string):Promise<Blob>{return (await api.request(`/api/attempts/${encodeURIComponent(id)}/handoff/arguments`,{method:"POST",headers:{"Content-Type":"application/json"},body:"{}"})).blob();}
};}
