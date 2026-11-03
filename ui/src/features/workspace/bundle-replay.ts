import type {Asset} from "../../api/assets";
import type {BundleUpload} from "../../api/bundle-imports";
import type {BundleSource} from "../../api/workspace";
import type {Declaration,Schema} from "../actions/model";
import type {PortBindings} from "../actions/ports";
import type {ImageSelection} from "../actions/selection";
export type UnresolvedInput={name:string;required:boolean;reasons:string[]};
export type ImportedDraft={source:BundleSource;tool:string;action:string;params:Schema;bindings:PortBindings;images:ImageSelection;assets:Asset[];unresolved:UnresolvedInput[]};
/** Data only: no preparation, estimate, provider call or archived approval can be replayed here. */
export async function importedDraft(upload:BundleUpload,tool:string,declaration:Declaration,readAsset:(id:string)=>Promise<Asset>):Promise<ImportedDraft>{
 const status=upload.import,step=status&&upload.record.manifest.steps[status.step];
 if(!status||status.phase==="importing"||status.bundle!==upload.record.id||status.tool!==tool||!step||step.tool.name!==declaration.document.tool.name||step.invocation.action!==status.action)throw Error("Import this step's inputs into the selected current installation before opening a draft.");
 const action=declaration.document.actions.find(a=>a.name===status.action);
 if(!action)throw Error("The imported action is no longer declared by this installation.");
 const operation=action.kind==="operation";
 const definitions=operation?action.ports?.in??[]:[...(action.images??[]).map(s=>({name:s.slot,type:"image",required:s.required,many:s.many})),...(action.locations??[]).map(s=>({name:s.slot,type:"location",required:s.required,many:false}))];
 const draft:ImportedDraft={source:{id:upload.record.id,step:status.step,...(status.replay?{replay:structuredClone(status.replay)}:{})},tool,action:status.action,params:structuredClone(status.params),bindings:{},images:{},assets:[],unresolved:[]};
 if(action.cost.may_spend)delete draft.params.max_spend_usd;
 const metadata=new Map<string,Asset>(),counts=new Map<string,number>();
 for(const item of status.inputs){
  const definition=definitions.find(p=>p.name===item.name),index=counts.get(item.name)??0;
  if(!definition||item.index!==index||(!definition.many&&index>0))throw Error("Imported input occurrences conflict with the current declaration.");
  counts.set(item.name,index+1);
  if(item.status!=="completed"||!item.binding){
   let unresolved=draft.unresolved.find(p=>p.name===item.name);
   if(!unresolved){unresolved={name:item.name,required:!!definition.required,reasons:[]};draft.unresolved.push(unresolved);}
   const reason=item.reason??"This input did not finish importing. Select a replacement explicitly.";if(!unresolved.reasons.includes(reason))unresolved.reasons.push(reason);
   continue;
  }
  const binding=structuredClone(item.binding);delete binding.origin;
  if(status.replay&&item.source)binding.origin=structuredClone(item.source);
  if(binding.kind==="image"){
   if(definition.type!=="image")throw Error("Imported image conflicts with its input type.");
   let asset=metadata.get(binding.asset);if(!asset){asset=await readAsset(binding.asset);metadata.set(binding.asset,asset);}
   if(operation)(draft.bindings[item.name]??=[]).push(binding);
   else(draft.images[item.name]??=[]).push({...asset,coordinates:binding.frame??{projection:"flat",topology:"bounded"},...(binding.origin?{origin:binding.origin}:{})});
  }else if(binding.kind==="location"){
   if(definition.type!=="location")throw Error("Imported shared location conflicts with its input type.");
   (draft.bindings[item.name]??=[]).push(binding);
  }else if(binding.kind==="artifact"&&operation&&binding.reference.type===definition.type&&binding.reference.tool===declaration.document.tool.name){
   (draft.bindings[item.name]??=[]).push(binding);
  }else throw Error("Imported native reference conflicts with its receiving tool or input type.");
 }
 for(const definition of definitions){if(definition.required&&!counts.has(definition.name))throw Error(`Imported step is missing required input ${definition.name}.`);}
 draft.assets=[...metadata.values()];return draft;
}
