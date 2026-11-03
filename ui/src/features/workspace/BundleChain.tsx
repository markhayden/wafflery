import {useEffect,useMemo,useRef,useState} from "react";
import type {LocalApi} from "../../api/client";
import {workspaceClient,type AttemptSummary} from "../../api/workspace";
import {bundleClient,type BundleRecord} from "../../api/bundles";
import {Button} from "../../components/Button";
import styles from "./InvocationBundles.module.css";
export function ChainDependencies({bundle}:{bundle:BundleRecord}){
 if(bundle.manifest.steps.length<2)return null;
 const chain=bundle.chain;
 return <section aria-label="Chain dependencies"><h4>Step connections</h4>
  {!chain||chain.problem?<p role="status">{chain?.problem??"Chain connections have not been verified."} Import individual steps or export a single-step bundle instead.</p>:<>
   {chain.edges.length?<ul>{chain.edges.map(e=><li key={`${e.consumer}:${e.name}:${e.index}`}>Step {e.producer.step+1} · {e.producer.port} item {e.producer.item+1} → Step {e.consumer+1} · {e.name} item {e.index+1}</li>)}</ul>:<p>These steps use external inputs; no earlier-output connections were found.</p>}
   <p>Connections must use freshly produced outputs on replay. Included native input copies preserve the original experiment; they do not replace those connections.</p>
  </>}
 </section>;
}
export function ChainSelection({api,onSaved,onBusy,disabled=false}:{api:LocalApi;disabled?:boolean;onSaved:(record:BundleRecord)=>Promise<void>;onBusy:(busy:boolean)=>void}){
 const client=useMemo(()=>workspaceClient(api),[api]);const bundles=useMemo(()=>bundleClient(api),[api]);const generation=useRef(0);
 const [items,setItems]=useState<AttemptSummary[]>([]),[selected,setSelected]=useState<AttemptSummary[]>([]),[cursor,setCursor]=useState<number|null>(null),[busy,setBusy]=useState(true),[notice,setNotice]=useState("");
 useEffect(()=>{const current=++generation.current;setBusy(true);setNotice("");void client.history().then(page=>{if(current===generation.current){setItems(page.items);setCursor(page.cursor);}}).catch(e=>{if(current===generation.current)setNotice(String(e));}).finally(()=>{if(current===generation.current)setBusy(false);});return()=>{generation.current++;};},[client]);
 async function more(){if(busy||disabled||cursor===null)return;const current=generation.current;setBusy(true);setNotice("");try{const page=await client.history(cursor);if(current===generation.current){setItems(previous=>[...previous,...page.items.filter(i=>!previous.some(p=>p.id===i.id))]);setCursor(page.cursor);}}catch(e){if(current===generation.current)setNotice(String(e));}finally{if(current===generation.current)setBusy(false);}}
 function move(index:number,offset:number){setSelected(previous=>{const next=[...previous];[next[index],next[index+offset]]=[next[index+offset]!,next[index]!];return next;});}
 async function save(){if(busy||disabled||!selected.length)return;const current=generation.current;setBusy(true);onBusy(true);setNotice("");try{const record=await bundles.createChain(selected.map(s=>s.id));if(current===generation.current)await onSaved(record);}catch(e){if(current===generation.current)setNotice(e instanceof Error?e.message:String(e));}finally{if(current===generation.current){setBusy(false);onBusy(false);}}}
 return <section aria-label="Chain selection" className={styles.bundles}>
  <p>Choose up to 16 saved runs in execution order. Only selected steps are included. Saving verifies connections against their recorded sources and runs nothing.</p>
  <fieldset disabled={busy||disabled}><legend>Saved runs</legend>{items.map(item=>{const checked=selected.some(s=>s.id===item.id);return <label className={styles.selectionLabel} key={item.id}><input type="checkbox" checked={checked} disabled={!checked&&selected.length===16} onChange={event=>setSelected(previous=>event.target.checked?[...previous,item]:previous.filter(s=>s.id!==item.id))}/><span>{item.tool} / {item.action} · {item.id.slice(0,8)} · {item.execution}</span></label>;})}{!busy&&!items.length&&<p>No saved runs are available.</p>}</fieldset>
  {cursor!==null&&<Button disabled={busy||disabled} onClick={()=>void more()}>Load older runs for selection</Button>}
  {!!selected.length&&<ol aria-label="Selected step order">{selected.map((item,index)=><li key={item.id}><span>{item.tool} / {item.action} · {item.id.slice(0,8)}</span><div className={styles.order}><Button disabled={busy||disabled||index===0} aria-label={`Move step ${index+1} up`} onClick={()=>move(index,-1)}>↑</Button><Button disabled={busy||disabled||index===selected.length-1} aria-label={`Move step ${index+1} down`} onClick={()=>move(index,1)}>↓</Button><Button disabled={busy||disabled} aria-label={`Remove step ${index+1}`} onClick={()=>setSelected(previous=>previous.filter(s=>s.id!==item.id))}>Remove</Button></div></li>)}</ol>}
  <Button disabled={busy||disabled||!selected.length} onClick={()=>void save()}>Save selected chain for review</Button>
  {busy&&<p role="status">Reading and verifying selected runs…</p>}{notice&&<p role="alert">{notice}</p>}
 </section>;
}
