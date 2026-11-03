import type {InputBinding} from "../../api/workspace";
export type Port={name:string;type:string;required?:boolean;many?:boolean;kinds?:string[]};
export type PortBindings=Record<string,InputBinding[]>;
export function moveBinding(items:InputBinding[],index:number,direction:number):InputBinding[]{
 const next=[...items],target=index+direction;
 if(index<0||index>=items.length||target<0||target>=items.length)return next;
 [next[index],next[target]]=[next[target]!,next[index]!];return next;
}
export function validatePorts(ports:Port[],bindings:PortBindings):string|undefined{
 for(const port of ports){const count=bindings[port.name]?.length??0;
 if(port.required&&!count)return `${port.name} requires ${port.many?"at least one item":"one item"}.`;
 if(!port.many&&count>1)return `${port.name} accepts only one item.`;
 }return undefined;
}
