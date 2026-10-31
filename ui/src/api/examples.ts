import type {LocalApi} from "./client";
import type {Attempt,Submission} from "./workspace";
export type Example={id:string;sequence:number;name:string;notes:string;request:Submission;inputs:Attempt["inputs"]};
export type ExampleSummary={id:string;sequence:number;name:string;tool:string;action:string};
export function exampleClient(api:LocalApi){
  async function read(response:Response):Promise<Example>{
    const value:unknown=await response.json();
    if(!value||typeof value!=="object"||!("id" in value)||typeof value.id!=="string"||!("request" in value)||!("inputs" in value)||!("name" in value)||typeof value.name!=="string")throw new Error("Example could not be read.");
    return value as Example;
  }
  return {
    async list(cursor?:number):Promise<{items:ExampleSummary[];cursor:number|null}>{
      const value=await(await api.request(`/api/examples${cursor===undefined?"":`?cursor=${cursor}`}`)).json() as {items:ExampleSummary[];cursor:number|null};
      if(!Array.isArray(value.items)||!value.items.every(item=>typeof item.id==="string"&&typeof item.name==="string"))throw new Error("Examples could not be read.");return value;
    },
    open:async(id:string)=>read(await api.request(`/api/examples/${encodeURIComponent(id)}`)),
    save:async(attempt_id:string,name:string,notes:string)=>read(await api.request("/api/examples",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({attempt_id,name,notes})})),
  };
}
