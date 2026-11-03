import {useEffect,useMemo,useRef,useState} from "react";
import {bundleImports,type BundleUpload,type ReviewSummary} from "../../api/bundle-imports";
import type {LocalApi} from "../../api/client";
import {workspaceClient,type RegisteredTool} from "../../api/workspace";
import {ChainSetup} from "./ChainReplays";
import type {ReplayStatus} from "../../api/chain-replays";
import {ChainDependencies} from "./BundleChain";
import {Button} from "../../components/Button";
import styles from "./InvocationBundles.module.css";

export function BundleImports({api,onOpen,onChainStarted}:{api:LocalApi;onOpen?:((upload:BundleUpload)=>Promise<void>)|undefined;onChainStarted?:((status:ReplayStatus)=>void)|undefined}){
 const client=useMemo(()=>bundleImports(api),[api]);const generation=useRef(0);
 const [reviews,setReviews]=useState<ReviewSummary[]>([]),[selected,setSelected]=useState<BundleUpload>(),[tools,setTools]=useState<RegisteredTool[]>([]),[step,setStep]=useState(0),[target,setTarget]=useState(""),[busy,setBusy]=useState(false),[loading,setLoading]=useState(true),[notice,setNotice]=useState("");
 useEffect(()=>{let active=true;++generation.current;setSelected(undefined);setNotice("");setReviews([]);setLoading(true);
  void Promise.all([client.list(),workspaceClient(api).snapshot()]).then(([items,snapshot])=>{if(active){setReviews(items);setTools(snapshot.tools);}}).catch(e=>{if(active)setNotice(String(e));}).finally(()=>{if(active)setLoading(false);});
  return()=>{active=false;generation.current++;};
 },[client,api]);
 const importing=selected?.import?.phase==="importing";
 useEffect(()=>{
  if(!selected||!importing)return;let active=true;let timer:ReturnType<typeof setTimeout>|undefined;const id=selected.id;
  async function poll(){try{const status=await client.progress(id);if(!active)return;setSelected(previous=>previous?.id===id?{...previous,import:status}:previous);if(status?.phase==="importing")timer=setTimeout(()=>void poll(),250);}catch(e){if(active)setNotice(`${String(e)} The owned import may still be running; reopen the review to reconcile.`);}}
  void poll();return()=>{active=false;if(timer)clearTimeout(timer);};
 },[client,selected?.id,importing]);
 const current=selected?.record.manifest.steps[step];
 const compatible=tools.filter(t=>t.declaration?.document.tool.name===current?.tool.name);
 const receiver=compatible.find(t=>t.id===target)??compatible[0];
 const action=receiver?.declaration?.document.actions.find(a=>a.name===current?.invocation.action);
 const locked=busy||importing||loading;
 async function operation(work:(current:number)=>Promise<void>){if(locked)return;const current=++generation.current;setBusy(true);setNotice("");try{await work(current);}catch(e){if(current===generation.current)setNotice(e instanceof Error?e.message:String(e));}finally{if(current===generation.current)setBusy(false);}}
 async function upload(file:File){await operation(async generationId=>{const review=await client.upload(file);if(generationId!==generation.current)return;setSelected(review);setStep(0);setTarget("");const items=await client.list();if(generationId===generation.current)setReviews(items);});}
 async function open(id:string){await operation(async generationId=>{const review=await client.read(id);if(generationId===generation.current){setSelected(review);setStep(review.import?.step??0);setTarget(review.import?.tool??"");}});}
 async function remove(id:string){await operation(async generationId=>{await client.remove(id);if(generationId!==generation.current)return;setReviews(previous=>previous.filter(i=>i.id!==id));setSelected(previous=>previous?.id===id?undefined:previous);});}
 async function start(){if(!selected||!receiver)return;const id=selected.id;await operation(async generationId=>{const progress=await client.start(id,step,receiver.id);if(generationId===generation.current)setSelected(previous=>previous?.id===id?{...previous,import:progress}:previous);});}
 async function openDraft(){if(!selected||!onOpen)return;await operation(async()=>{await onOpen(selected);});}
 return <section className={styles.bundles} aria-label="Bundle uploads">
  <p>Upload an uncompressed invocation tar (up to 256 MiB) to review it. Uploading verifies declared files; it does not install tools, change setup or run an action. Two reviews can remain open.</p>
  <label>Upload invocation bundle<input type="file" accept=".tar,application/x-tar" disabled={locked} onChange={event=>{const file=event.target.files?.[0];event.target.value="";if(file)void upload(file);}}/></label>
  {(busy||loading)&&<p role="status">{loading?"Reading uploaded reviews…":"Uploading or checking bundle data…"}</p>}
  {!!reviews.length&&<ul aria-label="Uploaded bundle list">{reviews.map(review=><li key={review.id}><Button disabled={locked} onClick={()=>void open(review.id)}>Review uploaded {review.bundle.slice(0,8)}</Button> <span>{selected?.id===review.id?selected.import?.phase??review.phase:review.phase}</span> <Button disabled={locked} onClick={()=>void remove(review.id)}>Dismiss {review.bundle.slice(0,8)}</Button></li>)}</ul>}
  {selected&&current&&<section className={styles.review} aria-label="Uploaded bundle review">
   <h3>Declared files verified</h3><p>Archive {selected.record.id.slice(0,12)} · {(selected.record.size/1024).toFixed(1)} KB. Current tool declarations still determine which inputs it can accept.</p>
   <ChainDependencies bundle={selected.record}/>{onChainStarted&&selected.record.manifest.steps.length>1&&<ChainSetup api={api} record={selected.record} uploadId={selected.id} onStarted={onChainStarted}/>}
   <label>Bundle step<select disabled={locked} value={step} onChange={event=>{setStep(Number(event.target.value));setTarget("");}}>{selected.record.manifest.steps.map((s,i)=><option key={i} value={i}>{i+1}. {s.tool.name} / {s.invocation.action}</option>)}</select></label>
   <p>Recorded tool version {current.tool.version}. No tool action has been run by this import flow.</p>
   <h4>Recorded requirements</h4>{current.invocation.requirements.length?<ul>{current.invocation.requirements.map((r,i)=><li key={i}>{r}</li>)}</ul>:<p>No additional requirements were recorded.</p>}
   {!!current.reference_only.length&&<><h4>Omitted inputs</h4><ul>{current.reference_only.map((r,i)=><li key={i}>{r.port??r.slot}: {r.why}</li>)}</ul></>}
   {!!current.unportable.length&&<><h4>Unportable inputs</h4><ul>{current.unportable.map((r,i)=><li key={i}>{r.port}: {r.reason}</li>)}</ul></>}
   <h4>Included files</h4><ul>{[...current.inputs,...current.artifacts].map((r,i)=><li key={i}><code title={r.member}>{r.member.replace(/([a-f0-9]{12})[a-f0-9]{52}/g,"$1…")}</code></li>)}</ul>
   <details><summary>Original invocation</summary><pre>{JSON.stringify(current.invocation,null,2)}</pre></details>
   <label>Receiving installation<select disabled={locked||compatible.length===0} value={receiver?.id??""} onChange={event=>setTarget(event.target.value)}>{!compatible.length&&<option value="">Tool is not installed</option>}{compatible.map(t=><option key={t.id} value={t.id}>{t.declaration!.document.tool.name} · {t.id}</option>)}</select></label>
   {!receiver&&<p>Install and register {current.tool.name} separately, then reopen this drawer. Uploading cannot install it.</p>}
   {receiver&&!action&&<p>The selected installation does not declare this action.</p>}
   {action&&<><p>Current action readiness: {action.readiness.state}. Configure requirements and effective file grants separately in Tool setup; import never changes them.</p>{action.readiness.requirements.length>0&&<ul>{action.readiness.requirements.map((r,i)=><li key={i}>{r.name} · {r.state}{r.setup?` · ${r.setup}`:""}</li>)}</ul>}</>}
   <Button disabled={locked||!receiver||!action} onClick={()=>void start()}>Import selected step inputs</Button>
   {selected.import&&<section aria-label="Input import progress"><p role="status">{importing?"Importing inputs. Closing the drawer keeps this work owned.":selected.import.phase==="failed"?"Import stopped. Completed inputs are retained; retry only when you choose.":"Inputs imported. Unresolved inputs still need explicit replacement before any Run."}</p><p>Step {selected.import.step+1} · {selected.import.tool} / {selected.import.action}</p><ul>{selected.import.inputs.map((i,n)=><li key={n}>{i.name} item {i.index+1} · {i.status}{i.reason?` · ${i.reason}`:""}</li>)}</ul>{selected.import.error&&<p role="alert">{selected.import.error}</p>}</section>}
   {onOpen&&selected.import&&selected.import.phase!=="importing"&&selected.import.step===step&&selected.import.tool===receiver?.id&&<Button disabled={locked} onClick={()=>void openDraft()}>Open as new draft</Button>}
   <p>Import retains the original archive and available input bytes. Omitted inputs stay unresolved. A new draft uses fresh readiness and estimates; opening it cannot replay upstream actions or authorize spending.</p>
  </section>}
  {notice&&<p role="alert">{notice}</p>}
 </section>;
}
