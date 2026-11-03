import {useEffect,useRef,useState} from "react";
import type {Attempt} from "../../api/workspace";
import type {Asset} from "../../api/assets";
import type {Json} from "../actions/model";
import {humanize} from "../actions/help";
import {artifactOccurrences} from "./artifacts";
import {Button} from "../../components/Button";
import {CodeBlock} from "../../components/CodeBlock";
import {Drawer} from "../../components/Drawer";
import styles from "./Workspace.module.css";

/** A run's tool outputs (artifacts: a mask, a marker…), one compact row each. The picture is in
 * the comparison above; the exact record opens in a drawer; "Use in another action" hands the
 * output to a later step as its input without running this one again. */
export function ArtifactOutputs({attempt,inspect,restore,onSend,disabled=false}:{attempt:Pick<Attempt,"id"|"result"|"outputs">;inspect:(index:number)=>Promise<Json>;restore?:((index:number)=>Promise<Json|undefined>)|undefined;onSend?:(index:number)=>void;disabled?:boolean;load?:(asset:Asset)=>Promise<Blob>}){
 const [view,setView]=useState<{index:number;data?:Json;error?:string}>();
 const generation=useRef(0);useEffect(()=>{generation.current++;setView(undefined);return()=>{generation.current++;};},[attempt.id]);
 async function open(index:number){const current=++generation.current;setView({index});try{const data=await inspect(index);if(current===generation.current)setView({index,data});}catch(error){if(current===generation.current)setView({index,error:error instanceof Error?error.message:"Artifact unavailable."});}}
 const items=artifactOccurrences(attempt.result);
 const shown=items.find(item=>item.index===view?.index);
 if(!items.length)return null;
 return <section aria-label="Operation outputs" className={styles.outputs}>
  <h3 className={styles.outputsTitle}>Outputs</h3>
  {items.map(item=><article key={item.index} className={styles.output}>
   <div><strong>{humanize(item.port)}{items.filter(other=>other.port===item.port).length>1?` ${item.index+1}`:""}</strong><span className={styles.note}> · {String(item.reference.type)}</span></div>
   <div className={styles.outputActions}>
    <Button type="button" aria-label={`Inspect ${item.port} item ${item.index+1}`} onClick={()=>void open(item.index)}>Details</Button>
    {onSend&&<Button type="button" disabled={disabled} aria-label={`Use ${item.port} item ${item.index+1} in another action`} onClick={()=>onSend(item.index)}>Use in another action…</Button>}
   </div>
  </article>)}
  <Drawer open={!!view} onClose={()=>setView(undefined)} title={shown?`${humanize(shown.port)} details`:"Output details"}
   description="The exact record the tool saved for this output. You don't need it to use the output; it identifies it precisely, for example in your own code.">
   {shown&&<p className={styles.note}>Identity · {String(shown.reference.identity)}</p>}
   {view&&(view.error?<><p role="alert">{view.error} The saved reference remains available; no run was started.</p>{restore&&shown&&<Button onClick={async()=>{const current=++generation.current;try{const data=await restore(shown.index);if(data&&current===generation.current)setView({index:shown.index,data});}catch(failure){if(current===generation.current)setView({index:shown.index,error:String(failure)});}}}>Restore {shown.port} item {shown.index+1}</Button>}</>
    :view.data!==undefined?<CodeBlock label="Artifact resource · native record is opaque" value={view.data}/>:<p role="status">Resolving saved artifact…</p>)}
  </Drawer>
 </section>;
}
