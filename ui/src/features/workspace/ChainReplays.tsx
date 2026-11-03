import {useEffect,useMemo,useRef,useState} from "react";
import type {LocalApi} from "../../api/client";
import {chainReplays,type ReplayStatus} from "../../api/chain-replays";
import {bundleClient,type BundleRecord} from "../../api/bundles";
import {workspaceClient,type RegisteredTool} from "../../api/workspace";
import {Button} from "../../components/Button";
import styles from "./InvocationBundles.module.css";
export function ChainSetup({api,record,uploadId,onStarted}:{api:LocalApi;record:BundleRecord;uploadId?:string|undefined;onStarted:(status:ReplayStatus)=>void}){
 const client=useMemo(()=>chainReplays(api),[api]),generation=useRef(0);
 const [tools,setTools]=useState<RegisteredTool[]>([]),[choices,setChoices]=useState<Record<string,string>>({}),[busy,setBusy]=useState(true),[notice,setNotice]=useState("");
 useEffect(()=>{const current=++generation.current;setBusy(true);setNotice("");setChoices({});void workspaceClient(api).snapshot().then(snapshot=>{if(current===generation.current)setTools(snapshot.tools);}).catch(e=>{if(current===generation.current)setNotice(String(e));}).finally(()=>{if(current===generation.current)setBusy(false);});return()=>{generation.current++;};},[api,record.id,uploadId]);
 const names=[...new Set(record.manifest.steps.map(s=>s.tool.name))];
 const available=(name:string)=>tools.filter(t=>t.declaration?.document.tool.name===name&&record.manifest.steps.filter(s=>s.tool.name===name).every(s=>t.declaration?.document.actions.some(a=>a.name===s.invocation.action)));
 const chosen=(name:string)=>choices[name]??available(name)[0]?.id??"";
 async function start(){if(busy)return;const current=generation.current;setBusy(true);setNotice("");try{const receivers=record.manifest.steps.map(s=>chosen(s.tool.name));if(receivers.some(id=>!id))throw Error("Register a matching installation for each named tool first.");const status=uploadId?await client.startUpload(uploadId,receivers):await client.start(record.id,receivers);if(current===generation.current)onStarted(status);}catch(e){if(current===generation.current)setNotice(e instanceof Error?e.message:String(e));}finally{if(current===generation.current)setBusy(false);}}
 if(record.manifest.steps.length<2)return null;
 return <section className={styles.bundles} aria-label="Start chain review"><h4>Review this chain in Wafflery</h4><p>Choose one installation per tool. Each step opens a normal draft for its own Run review. No step runs when this review starts, and paid steps need fresh caps and estimates.</p>
  {record.chain?.problem||!record.chain?<p role="status">{record.chain?.problem??"Step connections have not been verified."} Import individual steps instead.</p>:<fieldset disabled={busy}><legend>Receiving installations</legend>{names.map(name=><label key={name}>{name}<select aria-label={`${name} receiving installation`} value={chosen(name)} onChange={event=>setChoices(previous=>({...previous,[name]:event.target.value}))}>{!available(name).length&&<option value="">Matching tool/actions are not installed</option>}{available(name).map(tool=><option key={tool.id} value={tool.id}>{tool.id} · {tool.declaration!.document.tool.version}</option>)}</select></label>)}<Button disabled={names.some(name=>!chosen(name))} onClick={()=>void start()}>Start chain review</Button></fieldset>}
  {busy&&<p role="status">Reading installations or starting the review…</p>}{notice&&<p role="alert">{notice}</p>}
 </section>;
}
export function ChainReplays({api,activeId,onOpen}:{api:LocalApi;activeId?:string|undefined;onOpen?:((status:ReplayStatus)=>Promise<void>)|undefined}){
 const client=useMemo(()=>chainReplays(api),[api]),bundles=useMemo(()=>bundleClient(api),[api]);const generation=useRef(0),epoch=useRef(0),working=useRef(0);
 const [items,setItems]=useState<ReplayStatus[]>([]),[selected,setSelected]=useState<string>(),[record,setRecord]=useState<BundleRecord>(),[busy,setBusy]=useState(false),[loading,setLoading]=useState(true),[notice,setNotice]=useState("");
 useEffect(()=>{const current=++generation.current;let timer:ReturnType<typeof setTimeout>|undefined;setItems([]);setSelected(activeId);setLoading(true);setNotice("");working.current=0;epoch.current++;setBusy(false);
  async function poll(){const revision=epoch.current;try{if(!working.current){const list=await client.list();if(current!==generation.current)return;if(revision===epoch.current&&!working.current){setItems(list);setSelected(previous=>list.some(item=>item.id===previous)?previous:list.some(item=>item.id===activeId)?activeId:list[0]?.id);setLoading(false);}}}catch(e){if(current===generation.current){setNotice(String(e));setLoading(false);}}finally{if(current===generation.current)timer=setTimeout(()=>void poll(),500);}}
  void poll();return()=>{generation.current++;if(timer)clearTimeout(timer);};
 },[client,activeId]);
 const status=items.find(item=>item.id===selected);
 useEffect(()=>{let active=true;setRecord(undefined);if(status)void bundles.read(status.bundle).then(value=>{if(active)setRecord(value);}).catch(e=>{if(active)setNotice(String(e));});return()=>{active=false;};},[bundles,status?.bundle]);
 async function operation(work:()=>Promise<ReplayStatus|void>){if(working.current)return;const ticket=++epoch.current;working.current=ticket;const current=generation.current;setBusy(true);setNotice("");try{const value=await work();if(current===generation.current&&value)setItems(previous=>previous.map(item=>item.id===value.id?value:item));}catch(e){if(current===generation.current)setNotice(e instanceof Error?e.message:String(e));}finally{if(working.current===ticket){working.current=0;epoch.current++;if(current===generation.current)setBusy(false);}}}
 return <section className={styles.bundles} aria-label="Chain replays">
  <p>Review and run one step at a time. Closing this drawer keeps owned work running. Stop blocks later steps and cancels the current run; saved outcomes and charges remain.</p>
  {loading&&<p role="status">Reading chain reviews…</p>}{!loading&&!items.length&&<p>No chain reviews are open. Save or upload a chain to begin.</p>}
  {!!items.length&&<ul aria-label="Open chain reviews">{items.map(item=><li key={item.id}><Button disabled={busy} aria-pressed={selected===item.id} onClick={()=>{setSelected(item.id);setNotice("");}}>Review chain {item.id.slice(0,8)}</Button><span>{item.phase.replaceAll("_"," ")} · {item.step} / {item.total} steps completed</span></li>)}</ul>}
  {status&&<section className={styles.review} aria-label="Chain review"><h3>{status.phase==="completed"?"Chain completed":status.phase==="stopped"?"Chain stopped":`Step ${Math.min(status.step+1,status.total)} of ${status.total}`}</h3>
   <ol aria-label="Chain step progress">{status.tools.map((tool,index)=><li key={index} aria-current={index===status.step?"step":undefined}>{record?.manifest.steps[index]?.tool.name??tool} / {record?.manifest.steps[index]?.invocation.action??"Loading action…"}<span>{index<status.step?"Completed":index===status.step?status.phase.replaceAll("_"," "):"Waiting"}</span></li>)}</ol>
   {status.phase==="ready"&&<Button disabled={busy} onClick={()=>void operation(()=>client.stage(status.id))}>Prepare step {status.step+1}</Button>}
   {["preparing","importing","running"].includes(status.phase)&&<p role="status">{status.phase==="running"?"The ordinary run and its preservation are still owned. Wait for the saved outcome.":"Checking fresh outputs and importing this step's external inputs…"}</p>}
   {status.import&&<section aria-label="Chain input review"><h4>Current inputs</h4><ul>{status.import.inputs.map((input,index)=><li key={index}>{input.name} item {input.index+1} · {input.status}{input.changed&&<p>Output identity changed. This step will use the newly produced output.</p>}{input.reason&&<p>{input.reason}</p>}</li>)}</ul><details><summary>Current step parameters</summary><pre>{JSON.stringify(status.import.params,null,2)}</pre></details></section>}
   {status.phase==="awaiting_run"&&<><p>Inputs are ready for review. Opening the draft runs nothing. Choose a new cap and estimate for any paid action.</p>{onOpen&&<Button disabled={busy||!record} onClick={()=>void operation(async()=>{await onOpen(status);})}>Open step for review</Button>}</>}
   {status.error&&<p role="alert">{status.error}</p>}
   {["blocked","stopped"].includes(status.phase)&&<><p>Resume is explicit. Completed outputs are verified again; a failed step gets a new review, with fresh paid approval.</p><Button disabled={busy||status.settling} onClick={()=>void operation(()=>client.stage(status.id,true))}>Resume chain review</Button></>}
   {status.settling&&status.phase==="stopped"&&<p role="status">Waiting for owned input work or preservation to settle before resume.</p>}
   {status.attempts.length>0&&<section><h4>Saved attempts</h4><ul>{status.attempts.map(run=><li key={run.attempt}>Step {run.step+1} · {run.attempt.slice(0,8)} · {run.outcome}</li>)}</ul></section>}
   {!["stopped","completed"].includes(status.phase)&&<Button disabled={busy} onClick={()=>void operation(()=>client.stop(status.id))}>Stop chain</Button>}
   {["stopped","completed"].includes(status.phase)&&<Button disabled={busy||status.settling} onClick={()=>void operation(async()=>{const current=generation.current;await client.remove(status.id);if(current===generation.current){setItems(previous=>previous.filter(item=>item.id!==status.id));setSelected(undefined);}})}>Dismiss chain review</Button>}
  </section>}
  {notice&&<p role="alert">{notice}</p>}
 </section>;
}
