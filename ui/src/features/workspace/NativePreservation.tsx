import {useState} from "react";
import type {Attempt} from "../../api/workspace";
import {object} from "../actions/model";
import {Button} from "../../components/Button";
export function nativeEntries(attempt:Pick<Attempt,"observations">){
 const entries=object(attempt.observations)?.native_artifacts;
 return Array.isArray(entries)?entries.map(object).filter(entry=>entry!==undefined):[];
}
export function NativePreservation({attempt,retry}:{attempt:Pick<Attempt,"id"|"preservation"|"observations">;retry?:((id:string)=>Promise<unknown>)|undefined}){
 const [busy,setBusy]=useState(false),[error,setError]=useState<string>();
 const state=attempt.preservation.native;
 if(state==="not-needed")return null;
 return <section aria-label="Native preservation"><h3>Native artifacts · {state}</h3>
 <p>{state==="preserved"?"Verified native bytes are saved in this library. Restoring them to a tool store requires your confirmation.":state==="preserving"?"Saving reusable native bytes. This does not change the run outcome.":state==="unsupported"?"Native bytes were not preserved for this saved run.":"Some artifacts depend on the current tool store. The run outcome is unchanged."}</p>
 {nativeEntries(attempt).map((entry,index)=><p key={index}>{String(entry.port)} · {String(entry.direction)} item {Number(entry.index)+1} · {String(entry.state)}: {String(entry.reason)}</p>)}
 {retry&&(state==="failed"||state==="unsupported")&&<Button disabled={busy} onClick={async()=>{setBusy(true);setError(undefined);try{await retry(attempt.id);}catch(failure){setError(String(failure));}finally{setBusy(false);}}}>{busy?"Queuing preservation…":"Retry native preservation"}</Button>}
 {error&&<p role="alert">{error}</p>}
 </section>;
}
