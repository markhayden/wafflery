import {nativeEntries} from "./NativePreservation";
import {encodeSelection,type ImageSelection} from "../actions/selection";
import type {ImageSlot} from "../actions/model";
import type {Asset} from "../../api/assets";
import type {InputBinding,RegisteredTool,Attempt} from "../../api/workspace";
import {object,type Schema} from "../actions/model";
import type {ImageFrame} from "../actions/selection";
import {artifactOccurrences} from "./artifacts";
import {purposeParts} from "./activities";
/** A run's output, offered as a later action's input. `title` and `created` say which run made it;
 * `kept` is whether Wafflery saved a verified copy of a tool-native output (unset for outputs passed by value). */
export type ArtifactSource={title?:string;created?:number;kept?:boolean;reference:Schema;asset?:Pick<Asset,"id"|"sha256"|"width"|"height">|undefined;frame?:ImageFrame;location?:Schema;attempt?:string;port?:string;index?:number};
export function savedSources(attempt:Attempt):ArtifactSource[]{
 const occurrences=artifactOccurrences(attempt.result),locations=object(object(attempt.result)?.locations)?.found;
 return occurrences.map(item=>{
 const image=attempt.outputs.find(output=>`sha256:${output.asset.sha256}`===item.reference.identity);
 const location=item.reference.type==="location"&&occurrences.filter(item=>item.reference.type==="location").length===1&&Array.isArray(locations)&&locations.length===1?object(locations[0]):undefined;
 const native=nativeEntries(attempt).find(entry=>entry.direction==="out"&&entry.port===item.port&&entry.index===item.index);
 const title=attempt.declaration?.document?.actions?.find(action=>action.name===attempt.request.action)?.title??attempt.request.action;
 return {title,created:attempt.created_ms,...(native?{kept:native.state==="preserved"}:{}),reference:item.reference,attempt:attempt.id,port:item.port,index:item.index,...(image?{asset:image.asset,frame:{projection:image.image.projection,topology:image.image.topology} as ImageFrame}:{}),...(location?{location}:{})};
 });
}
export function transferBinding(source:ArtifactSource,tool:string):InputBinding|undefined{
 const binding=transferValue(source,tool);return binding&&source.attempt!==undefined&&source.port!==undefined&&source.index!==undefined?{...binding,origin:{attempt:source.attempt,port:source.port,occurrence:source.index}}:binding;
}
function transferValue(source:ArtifactSource,tool:string):InputBinding|undefined{
 if(source.reference.tool===tool)return {kind:"artifact",reference:structuredClone(source.reference)};
 if(source.reference.type==="image"&&source.asset&&source.reference.identity===`sha256:${source.asset.sha256}`)return {kind:"image",asset:source.asset.id,...(source.frame?{frame:source.frame}:{})};
 if(source.reference.type==="location"&&source.location)return {kind:"location",value:structuredClone(source.location)};
 return undefined;
}
/** Where a source can go: every operation port that takes its type, with what the card shows.
 * An action with several such ports gets one card per port, each saying which role the source
 * takes there. */
export type Destination={tool:string;action:string;port:string;many:boolean;title:string;description:string;group?:string;toolName:string};
export function compatibleDestinations(source:ArtifactSource,tools:RegisteredTool[]):Destination[]{
 return tools.flatMap(tool=>{
  const document=tool.declaration?.document;if(!document||!transferBinding(source,document.tool.name))return [];
  return document.actions.filter(action=>action.kind==="operation").flatMap(action=>{
   const ports=(action.ports?.in??[]).filter(port=>port.type===source.reference.type);
   return ports.map(port=>({tool:tool.id,action:action.name,port:port.name,many:!!port.many,toolName:document.tool.name,
    title:ports.length>1?`${action.title} · as ${port.name}`:action.title,
    description:ports.length>1&&port.purpose?port.purpose:purposeParts(action.purpose).summary,
    ...(action.group?{group:action.group}:{})}));
  });
 });
}

export function encodePortBindings(bindings:Record<string,InputBinding[]>){
 const lineage:Record<string,(import("../../api/workspace").BindingOrigin|null)[]>={};
 const values=Object.fromEntries(Object.entries(bindings).map(([port,items])=>{
 if(items.some(item=>item.origin))lineage[port]=items.map(item=>item.origin??null);
 return [port,items.map(({origin:_,...value})=>value)];
 }));return {bindings:values,lineage};
}

export function encodeWorkspaceInputs(version:string,slots:ImageSlot[],images:ImageSelection,ports:Record<string,InputBinding[]>){
 const selected=encodeSelection(version,slots,images),bound=encodePortBindings(ports);
 return {...selected,...bound,lineage:{...selected.lineage,...bound.lineage}};
}
