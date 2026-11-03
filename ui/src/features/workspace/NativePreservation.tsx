import {useState} from "react";
import type {Attempt} from "../../api/workspace";
import {object} from "../actions/model";
import {Button} from "../../components/Button";
import styles from "./Workspace.module.css";
export function nativeEntries(attempt:Pick<Attempt,"observations">){
 const entries=object(attempt.observations)?.native_artifacts;
 return Array.isArray(entries)?entries.map(object).filter(entry=>entry!==undefined):[];
}
export function NativePreservation({attempt,retry}:{attempt:Pick<Attempt,"id"|"preservation"|"observations">;retry?:((id:string)=>Promise<unknown>)|undefined}){
 const [busy,setBusy]=useState(false),[error,setError]=useState<string>();
 const state=attempt.preservation.native;
 if(state==="not-needed")return null;
 // In plain words: besides the images, a tool can keep its own full record of an output (rascal:
 // a mask's zones and how it was made). Wafflery copies that record into your library so the
 // output can be reused later even if the tool's own copy is cleared.
 const failed=state==="failed"||state==="unsupported";
 const reasons=nativeEntries(attempt).filter(entry=>entry.state!=="preserved");
 return <section aria-label="Native preservation" className={failed?styles.preserveFailed:styles.preserve}>
 <h3>{state==="preserved"?"Saved for reuse":state==="preserving"?"Saving for reuse…":"Not saved for reuse"}<span className={styles.hidden}> · Native artifacts · {state}</span></h3>
 <p>{state==="preserved"?"The tool's full record of these outputs is in your library, so you can reuse them later even if the tool's own copy is cleared."
  :state==="preserving"?"Copying the tool's full record of these outputs into your library. The result above is already complete."
  :"The result above is complete and its images are saved. The tool's full record of these outputs could not be copied into your library, so reusing them later needs the tool's own copy to still exist."}</p>
 {failed&&reasons.length>0&&<details><summary>Why</summary>{reasons.map((entry,index)=><p key={index} className={styles.note}>{String(entry.port)} · {String(entry.direction)} item {Number(entry.index)+1}: {String(entry.reason)}</p>)}</details>}
 {retry&&failed&&<Button disabled={busy} onClick={async()=>{setBusy(true);setError(undefined);try{await retry(attempt.id);}catch(failure){setError(String(failure));}finally{setBusy(false);}}}>{busy?"Trying again…":"Try again"}</Button>}
 {error&&<p role="alert">{error}</p>}
 </section>;
}
