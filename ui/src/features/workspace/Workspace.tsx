import {exampleClient,type Example} from "../../api/examples";
import {Examples} from "./Examples";
import {watchWorkspace} from "../../api/events";
import {useEffect,useMemo,useRef,useState} from "react";
import type {LocalApi} from "../../api/client";
import {assetClient,type Asset} from "../../api/assets";
import {workspaceClient,type Attempt,type Snapshot,type Submission} from "../../api/workspace";
import {initialParameters,encodeBindings,type Schema} from "../actions/model";
import {ActionForm} from "../actions/ActionForm";
import {Button} from "../../components/Button";
import {Picker} from "../../components/Picker";
import {Drawer} from "../../components/Drawer";
import {SavedComparison} from "./SavedComparison";
import {History} from "./History";
import {Inspector} from "./Inspector";
import styles from "./Workspace.module.css";
export function Workspace({api,initial}:{api:LocalApi;initial:Snapshot}) {
 const [snapshot,setSnapshot]=useState(initial);
 const [connection,setConnection]=useState("");
 const client=useMemo(()=>workspaceClient(api),[api]),assets=useMemo(()=>assetClient(api),[api]);
 const examples=useMemo(()=>exampleClient(api),[api]);
 const [draftExample,setDraftExample]=useState<Example>();
 const preferred=new URLSearchParams(window.location.search).get("tool");
 const [toolId,setToolId]=useState(snapshot.tools.find(t=>t.id===preferred)?.id??snapshot.tools[0]?.id??"");
 const tool=snapshot.tools.find(tool=>tool.id===toolId);const declaration=tool?.declaration;
 const [actionId,setActionId]=useState(declaration?.document.actions[0]?.name??"");
 const action=declaration?.document.actions.find(a=>a.name===actionId);
 const [params,setParams]=useState<Schema>(()=>initialParameters(action?.params_schema??{}));
 const [projections,setProjections]=useState<Record<string,"flat"|"equirectangular">>({});
 const [images,setImages]=useState<Record<string,Asset[]>>({});const [savedImages,setSavedImages]=useState<Asset[]>([]);
 const [selected,setSelected]=useState<Attempt|undefined>();
 const recent=snapshot.attempts;const [selectedId,setSelectedId]=useState<string|null>();const [busy,setBusy]=useState(false),[error,setError]=useState<string>();
 const [drawer,setDrawer]=useState<{opener:HTMLElement;view:"inspect"|"history"|"examples"}|null>(null);const submitted=useRef<{body:string;request:Submission}|null>(null);
 useEffect(()=>{void assets.list().then(setSavedImages).catch(()=>{});},[assets]);
 useEffect(()=>{
  const controller=new AbortController();
  if(initial.event_session)void watchWorkspace(api,initial.event_session,async()=>{
   const next=await client.snapshot();if(controller.signal.aborted)return;
   setSnapshot(next);
  },message=>{if(!controller.signal.aborted)setConnection(message);},controller.signal);
  return()=>controller.abort();
 },[api,client,initial.event_session]);
 useEffect(()=>{
  let active=true;
  const id=selectedId===null?undefined:selectedId??snapshot.attempts.find(a=>a.tool===toolId&&a.action===actionId)?.id;
  if(!id){setSelected(undefined);return;}
  void client.attempt(id).then(attempt=>{if(active)setSelected(attempt);}).catch(failure=>{if(active){setSelected(undefined);setError(failure instanceof Error?failure.message:"Unable to load run.");}});
  return()=>{active=false;};
 },[client,snapshot,selectedId,toolId,actionId]);
 function openAttempt(id:string){
  void client.attempt(id).then(attempt=>{
   const action=snapshot.tools.find(t=>t.id===attempt.request.tool)?.declaration?.document.actions.find(a=>a.name===attempt.request.action);
   setToolId(attempt.request.tool);setActionId(attempt.request.action);setParams(initialParameters(action?.params_schema??{}));setImages({});setProjections({});
   setSelectedId(id);setSelected(attempt);setDrawer(null);
  }).catch(failure=>setError(String(failure)));
 }
 function openExample(example:Example){
  const current=snapshot.tools.find(tool=>tool.id===example.request.tool)?.declaration;
  if(!current?.document.actions.some(action=>action.name===example.request.action))throw new Error("This example needs a tool action that is not currently available.");
  setToolId(example.request.tool);setActionId(example.request.action);setParams(structuredClone(example.request.params));
  setImages(Object.fromEntries(Object.entries(example.inputs).map(([slot,values])=>[slot,values.map(value=>value.asset)])));
  setProjections(Object.fromEntries(Object.entries(example.request.frames??{}).map(([slot,values])=>[slot,values[0]?.projection??"flat"])));
  setSelectedId(null);setSelected(undefined);setDraftExample(example);setError(undefined);submitted.current=null;setDrawer(null);
 }
 function chooseTool(id:string){setDraftExample(undefined);setToolId(id);const action=snapshot.tools.find(t=>t.id===id)?.declaration?.document.actions[0];setActionId(action?.name??"");setParams(initialParameters(action?.params_schema??{}));setImages({});setProjections({});setSelectedId(undefined);setSelected(undefined);setError(undefined);submitted.current=null;}
 function chooseAction(id:string){setDraftExample(undefined);setActionId(id);setParams(initialParameters(declaration?.document.actions.find(a=>a.name===id)?.params_schema??{}));setImages({});setProjections({});setSelectedId(undefined);setSelected(undefined);setError(undefined);submitted.current=null;}
 async function run(){
  if(!action||!declaration||busy)return;setError(undefined);setBusy(true);
  try{
   const bindings=encodeBindings(declaration.document.contract,action.images??[],Object.fromEntries(Object.entries(images).map(([slot,values])=>[slot,values.map(a=>a.id)])));
   const frames=Object.fromEntries(Object.entries(bindings).map(([slot,values])=>[slot,values.map(()=>({projection:projections[slot]??"flat",topology:projections[slot]==="equirectangular"?"wrap" as const:"bounded" as const}))]));
   const body=JSON.stringify({tool:toolId,action:actionId,revision:declaration.revision,params,images:bindings,frames});
   if(submitted.current?.body!==body)submitted.current={body,request:{id:crypto.randomUUID(),tool:toolId,action:actionId,revision:declaration.revision,params:structuredClone(params),images:bindings,frames}};
   const attempt=await client.submit(submitted.current.request);setSelectedId(attempt.id);setSelected(attempt);setSnapshot(await client.snapshot());submitted.current=null;
  }catch(failure){setError(failure instanceof Error?failure.message:"The attempt could not be submitted.");}finally{setBusy(false);}
 }
 const working=selected&&(["submitted","running","unconfirmed","cancelling"].includes(selected.execution)||selected.preservation.contract==="pending");
 return <div className={styles.workspace}>
  <div><Button onClick={event=>setDrawer({opener:event.currentTarget,view:"history"})}>History</Button> <Button onClick={event=>setDrawer({opener:event.currentTarget,view:"examples"})}>Examples</Button></div>
  <div className={styles.pickers}><Picker label="Tool" value={toolId} onChange={chooseTool} choices={snapshot.tools.map(t=>({id:t.id,title:t.declaration?.document.tool.name??t.id,description:t.error??`Version ${t.declaration?.document.tool.version??"unknown"}`}))}/><Picker label="Action" value={actionId} onChange={chooseAction} disabled={!declaration} choices={(declaration?.document.actions??[]).map(a=>({id:a.name,title:a.title,description:(a.images??[]).length?"Image → Result":"Parameters → Result"}))}/></div>
  {declaration&&<p className={styles.note}>{declaration.document.tool.purpose}</p>}
  {connection&&<p role="status">{connection}</p>}
  {tool?.error&&<p role="alert">{tool.error}</p>}
  <div className={styles.columns}>
   <form className={styles.panel} onSubmit={event=>{event.preventDefault();void run();}}><h2>Input</h2>{draftExample&&<p className={styles.note}>From example: {draftExample.name}{draftExample.notes?` · ${draftExample.notes}`:""}</p>}
    {action?<ActionForm key={`${toolId}/${actionId}`} action={action} params={params} onChange={setParams} projections={projections} onProjection={(slot,value)=>setProjections(values=>({...values,[slot]:value}))} images={images} onImages={(slot,value)=>setImages(values=>({...values,[slot]:value}))} upload={async file=>{const asset=await assets.upload(file);setSavedImages(await assets.list());return asset;}} preview={assets.preview}/>:<p>No supported action is available.</p>}
    {action?.images?.length&&savedImages.length>0?<details><summary>Saved images ({savedImages.length})</summary><div className={styles.saved}>{action.images.map(slot=><div key={slot.slot}><p>Use for {slot.slot}</p>{savedImages.slice(0,12).map((asset,index)=><Button key={asset.id} onClick={()=>setImages(values=>({...values,[slot.slot]:[asset]}))}>Image {index+1} · {asset.width} × {asset.height}</Button>)}</div>)}</div></details>:null}
    {error&&<p role="alert">{error}</p>}<div className={styles.run}><p>{action?.cost.may_spend?"Paid actions are unavailable in this foundation.":"Free action · Inputs are saved locally"}</p><Button type="submit" busy={busy} disabled={!action||action.cost.may_spend||!!working||!!tool?.busy||(action.images??[]).some(slot=>slot.required&&!images[slot.slot]?.length)||(action.locations??[]).some(slot=>slot.required)}>Run</Button></div>
   </form>
   <section className={styles.panel} aria-label="Result"><div className={styles.resultHeading}><h2>Result</h2><span role="status">{selected?(selected.execution==="unconfirmed"?"Cancellation requested · outcome unconfirmed":selected.execution):"Ready"}</span></div>
    {selected&&["submitted","running"].includes(selected.execution)&&<Button onClick={()=>{void client.cancel(selected.id).then(setSelected).catch(failure=>setError(failure instanceof Error?failure.message:"Unable to request cancellation."));}}>Cancel run</Button>}
    {selected?<>{working&&tool?.progress?.message&&<p className={styles.note}>{tool.progress.message}</p>}<SavedComparison attempt={selected} client={client} load={assets.preview} revision={snapshot}/><div className={styles.resultHeading}><p>Saved attempt · {selected.id.slice(0,8)}</p><Button onClick={event=>setDrawer({opener:event.currentTarget,view:"inspect"})}>Inspect run</Button></div><p className={styles.note}>Contract evidence: {selected.preservation.contract}. Editing the inputs starts a new draft.</p>{selected.notices.map((notice,index)=><p className={styles.note} key={index}>{notice}</p>)}</>:<div className={styles.empty}><p>Choose your inputs and run an action.</p><p>The result and its saved evidence will appear here.</p></div>}
   </section>
  </div>
  {recent.length>0&&<section className={styles.recent} aria-label="Recent attempts"><h2>Recent attempts</h2>{recent.slice(0,6).map(attempt=><Button key={attempt.id} onClick={()=>openAttempt(attempt.id)}>{attempt.action} · {attempt.execution} · {attempt.id.slice(0,8)}</Button>)}</section>}
  <Drawer open={drawer!==null} opener={drawer?.opener??null} onClose={()=>setDrawer(null)} title={drawer?.view==="history"?"History":drawer?.view==="examples"?"Examples":"Run details"} description={drawer?.view==="history"?"Saved local attempts. Opening a run never executes it.":drawer?.view==="examples"?"Reusable inputs, saved locally.":selected?`${selected.request.tool} / ${selected.request.action} · ${selected.id}`:""}>{drawer?.view==="history"?<History client={client} onOpen={openAttempt}/>:drawer?.view==="examples"?<Examples client={examples} selectedId={selected?.id} onOpen={openExample}/>:selected&&<Inspector key={selected.id} attempt={selected}/>}</Drawer>
 </div>;
}
