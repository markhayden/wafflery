import {object,type Json,type Schema} from "../actions/model";
export type ArtifactOccurrence={index:number;port:string;reference:Schema;preview:Schema|undefined};
export function artifactOccurrences(result:Json):ArtifactOccurrence[]{
 const entries=object(object(result)?.artifacts)?.out;
 if(!Array.isArray(entries))return [];
 return entries.flatMap((entry,index)=>{const value=object(entry),reference=object(value?.artifact);
 return value&&typeof value.port==="string"&&reference&&typeof reference.type==="string"&&typeof reference.identity==="string"&&typeof reference.resource==="string"&&typeof reference.tool==="string"?[{index,port:value.port,reference,preview:object(value.preview)}]:[];});
}
