import type {Asset} from "../../api/assets";
import type {InputBinding,RegisteredTool,Attempt} from "../../api/workspace";
import {object,type Schema} from "../actions/model";
import type {ImageFrame} from "../actions/selection";
import {artifactOccurrences} from "./artifacts";
export type ArtifactSource={reference:Schema;asset?:Pick<Asset,"id"|"sha256"|"width"|"height">|undefined;frame?:ImageFrame;location?:Schema;attempt?:string;port?:string;index?:number};
export function savedSources(attempt:Attempt):ArtifactSource[]{
 const occurrences=artifactOccurrences(attempt.result),locations=object(object(attempt.result)?.locations)?.found;
 return occurrences.map(item=>{
 const image=attempt.outputs.find(output=>`sha256:${output.asset.sha256}`===item.reference.identity);
 const location=item.reference.type==="location"&&occurrences.filter(item=>item.reference.type==="location").length===1&&Array.isArray(locations)&&locations.length===1?object(locations[0]):undefined;
 return {reference:item.reference,attempt:attempt.id,port:item.port,index:item.index,...(image?{asset:image.asset,frame:{projection:image.image.projection,topology:image.image.topology} as ImageFrame}:{}),...(location?{location}:{})};
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
export function compatibleDestinations(source:ArtifactSource,tools:RegisteredTool[]){
 return tools.flatMap(tool=>(tool.declaration?.document.actions??[]).filter(action=>action.kind==="operation").flatMap(action=>(action.ports?.in??[]).filter(port=>port.type===source.reference.type&&!!transferBinding(source,tool.declaration!.document.tool.name)).map(port=>({tool:tool.id,action:action.name,port:port.name,many:!!port.many,title:`${tool.declaration!.document.tool.name} / ${action.title} / ${port.name}`}))));
}

export function encodePortBindings(bindings:Record<string,InputBinding[]>){
 const lineage:Record<string,(import("../../api/workspace").BindingOrigin|null)[]>={};
 const values=Object.fromEntries(Object.entries(bindings).map(([port,items])=>{
 if(items.some(item=>item.origin))lineage[port]=items.map(item=>item.origin??null);
 return [port,items.map(({origin:_,...value})=>value)];
 }));return {bindings:values,lineage};
}
