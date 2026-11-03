import {useEffect,useRef,useState} from "react";
import type {InputBinding,workspaceClient} from "../../api/workspace";
import {Drawer} from "../../components/Drawer";
import {Button} from "../../components/Button";
import {nativeEntries} from "./NativePreservation";
type Pending={binding:InputBinding;target:string;message:string;owned:boolean;finish:(value:InputBinding|undefined)=>void};
export function useArtifactRestore(client:ReturnType<typeof workspaceClient>,context:string){
 const [pending,setPending]=useState<Pending>(),[busy,setBusy]=useState(false);
 const current=useRef<Pending|undefined>(undefined),generation=useRef(0);
 function finish(value?:InputBinding){const saved=current.current;current.current=undefined;setPending(undefined);setBusy(false);saved?.finish(value);}
 useEffect(()=>()=>{generation.current++;const saved=current.current;current.current=undefined;saved?.finish(undefined);},[context]);
 // A context change invalidates pending choices rather than writing into a different draft.
 useEffect(()=>{setPending(undefined);setBusy(false);},[context]);
 async function choose(binding:InputBinding,target:string):Promise<InputBinding|undefined>{
  if(binding.kind!=="artifact"||!binding.origin)return binding;
  const previous=current.current;current.current=undefined;setPending(undefined);previous?.finish(undefined);
  const version=++generation.current,origin=binding.origin;
  try{await client.artifact(origin.attempt,origin.occurrence,target);return version===generation.current?binding:undefined;}
  catch(failure){
   let owned=false,message=String(failure);
   try{const saved=await client.attempt(origin.attempt);const entry=nativeEntries(saved).find(entry=>entry.direction==="out"&&entry.index===origin.occurrence);owned=entry?.state==="preserved";if(!owned&&entry)message+=` ${String(entry.reason)}`;}catch{/* Keep the resource refusal visible. */}
   if(version!==generation.current)return;
   return new Promise(resolve=>{const value={binding,target,message,owned,finish:resolve};current.current=value;setPending(value);});
  }
 }
 async function restore(importBytes:boolean){
  const value=current.current;if(!value||!value.binding.origin)return;setBusy(true);
  try{const origin=value.binding.origin;
   if(importBytes)await client.importArtifact(origin.attempt,origin.occurrence,value.target);
   // An import acknowledgement alone is not proof that the receiving store resolves this reference.
   await client.artifact(origin.attempt,origin.occurrence,value.target);
   if(current.current===value)finish(value.binding);
  }catch(failure){if(current.current===value){const changed={...value,message:String(failure)};current.current=changed;setPending(changed);setBusy(false);}}
 }
 const drawer=<Drawer open={!!pending} title="Restore saved artifact" onClose={()=>{generation.current++;finish();}} description="No binding is added until the current tool store resolves the saved identity. No action will run.">
  {pending&&<><p role="alert">{pending.message}</p><p>{pending.owned?"Verified native bytes are saved in this library. Import a copy into the receiving tool store to continue.":"No verified native copy is recorded. Retry preservation from the saved run while its original store is available."}</p>
  <div style={{display:"flex",flexWrap:"wrap",gap:8}}>{pending.owned&&<Button disabled={busy} onClick={()=>void restore(true)}>Import saved copy</Button>}
  <Button disabled={busy} onClick={()=>void restore(false)}>Check availability again</Button>
  <Button onClick={()=>{generation.current++;finish();}}>Cancel selection</Button></div>
  {busy&&<p role="status">Restoring and verifying the saved identity…</p>}</>}
 </Drawer>;
 return {choose,drawer};
}
