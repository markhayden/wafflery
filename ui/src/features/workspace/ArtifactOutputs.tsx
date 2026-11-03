import {useEffect,useRef,useState} from "react";
import type {Attempt, SavedImage} from "../../api/workspace";
import type {Asset} from "../../api/assets";
import {object,type Json} from "../actions/model";
import {artifactOccurrences} from "./artifacts";
import {Button} from "../../components/Button";
import styles from "./Workspace.module.css";
export function ArtifactOutputs({attempt,inspect,onSend,load,disabled=false}:{attempt:Pick<Attempt,"id"|"result"|"outputs">;inspect:(index:number)=>Promise<Json>;onSend?:(index:number)=>void;disabled?:boolean;load?:(asset:Asset)=>Promise<Blob>}){
 const [view,setView]=useState<{index:number;data?:Json;error?:string}>();
 const generation=useRef(0);useEffect(()=>{generation.current++;setView(undefined);return()=>{generation.current++;};},[attempt.id]);
 async function open(index:number){const current=++generation.current;setView({index});try{const data=await inspect(index);if(current===generation.current)setView({index,data});}catch(error){if(current===generation.current)setView({index,error:error instanceof Error?error.message:"Artifact unavailable."});}}
 return <section aria-label="Operation outputs">{artifactOccurrences(attempt.result).map(item=><article key={item.index}>
 <h3>{item.port} · {String(item.reference.type)}</h3>
 <p className={styles.note}>{String(item.reference.identity)}</p>
 {item.preview&&<ArtifactPreview saved={attempt.outputs.find(output=>output.asset.sha256===object(item.preview?.image)?.file_sha256)} load={load} />}
 <p>{item.preview?"Preview is a visual aid; the reference identifies this artifact.":"No preview supplied."}</p>
 <Button type="button" onClick={()=>void open(item.index)}>Inspect {item.port} item {item.index+1}</Button>
 {onSend&&<Button type="button" disabled={disabled} onClick={()=>onSend(item.index)}>Send {item.port} item {item.index+1} to…</Button>}
 {view?.index===item.index&&(view.error?<p role="alert">{view.error} The saved reference remains available; no run was started.</p>:view.data?<details open><summary>Artifact resource · native record is opaque</summary><pre style={{whiteSpace:"pre-wrap",overflowWrap:"anywhere"}}>{JSON.stringify(view.data,null,2)}</pre></details>:<p role="status">Resolving saved artifact…</p>)}
 </article>)}</section>;
}

function ArtifactPreview({saved,load}:{saved:SavedImage|undefined;load:((asset:Asset)=>Promise<Blob>)|undefined}){
 const [url,setUrl]=useState<string>(),[failed,setFailed]=useState(false);
 useEffect(()=>{let active=true,owned:string|undefined;setUrl(undefined);setFailed(false);if(saved&&load)void load(saved.asset).then(blob=>{if(active){owned=URL.createObjectURL(blob);setUrl(owned);}}).catch(()=>{if(active)setFailed(true);});return()=>{active=false;if(owned)URL.revokeObjectURL(owned);};},[saved,load]);
 return url?<img src={url} alt="Artifact preview" style={{maxWidth:"100%",maxHeight:280,objectFit:"contain"}}/>:<p>{!saved||failed?"Preview unavailable. The artifact reference is unchanged.":"Loading preview…"}</p>;
}
