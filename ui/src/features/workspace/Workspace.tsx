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
import {ImageCompare,type CompareImage} from "../../components/ImageCompare";
import {Inspector} from "./Inspector";
import styles from "./Workspace.module.css";
export function Workspace({api,initial}:{api:LocalApi;initial:Snapshot}) {
 const [snapshot,setSnapshot]=useState(initial);
 const [connection,setConnection]=useState("");
 const client=useMemo(()=>workspaceClient(api),[api]),assets=useMemo(()=>assetClient(api),[api]);
 const preferred=new URLSearchParams(window.location.search).get("tool");
 const [toolId,setToolId]=useState(snapshot.tools.find(t=>t.id===preferred)?.id??snapshot.tools[0]?.id??"");
 const tool=snapshot.tools.find(tool=>tool.id===toolId);const declaration=tool?.declaration;
 const [actionId,setActionId]=useState(declaration?.document.actions[0]?.name??"");
 const action=declaration?.document.actions.find(a=>a.name===actionId);
 const [params,setParams]=useState<Schema>(()=>initialParameters(action?.params_schema??{}));
 const [images,setImages]=useState<Record<string,Asset[]>>({});const [savedImages,setSavedImages]=useState<Asset[]>([]);
 const [selected,setSelected]=useState<Attempt|undefined>(snapshot.attempts.find(a=>a.request.tool===toolId));
 const [recent,setRecent]=useState(snapshot.attempts);const [busy,setBusy]=useState(false),[error,setError]=useState<string>();
 const [drawer,setDrawer]=useState<{opener:HTMLElement}|null>(null);const submitted=useRef<{body:string;request:Submission}|null>(null);
 useEffect(()=>{void assets.list().then(setSavedImages).catch(()=>{});},[assets]);
 useEffect(()=>{
  const controller=new AbortController();
  if(initial.event_session)void watchWorkspace(api,initial.event_session,async()=>{
   const next=await client.snapshot();if(controller.signal.aborted)return;
   setSnapshot(next);setRecent(next.attempts);setSelected(previous=>next.attempts.find(a=>a.id===previous?.id)??previous??next.attempts[0]);
  },message=>{if(!controller.signal.aborted)setConnection(message);},controller.signal);
  return()=>controller.abort();
 },[api,client,initial.event_session]);
 function chooseTool(id:string){setToolId(id);const action=snapshot.tools.find(t=>t.id===id)?.declaration?.document.actions[0];setActionId(action?.name??"");setParams(initialParameters(action?.params_schema??{}));setImages({});setSelected(undefined);setError(undefined);submitted.current=null;}
 function chooseAction(id:string){setActionId(id);setParams(initialParameters(declaration?.document.actions.find(a=>a.name===id)?.params_schema??{}));setImages({});setSelected(undefined);setError(undefined);submitted.current=null;}
 async function run(){
  if(!action||!declaration||busy)return;setError(undefined);setBusy(true);
  try{
   const bindings=encodeBindings(declaration.document.contract,action.images??[],Object.fromEntries(Object.entries(images).map(([slot,values])=>[slot,values.map(a=>a.id)])));
   const body=JSON.stringify({tool:toolId,action:actionId,revision:declaration.revision,params,images:bindings});
   if(submitted.current?.body!==body)submitted.current={body,request:{id:crypto.randomUUID(),tool:toolId,action:actionId,revision:declaration.revision,params:structuredClone(params),images:bindings}};
   const attempt=await client.submit(submitted.current.request);setSelected(attempt);setRecent(values=>[attempt,...values.filter(value=>value.id!==attempt.id)]);submitted.current=null;
  }catch(failure){setError(failure instanceof Error?failure.message:"The attempt could not be submitted.");}finally{setBusy(false);}
 }
 const working=selected&&(["submitted","running","unconfirmed","cancelling"].includes(selected.execution)||selected.preservation.contract==="pending");
 const source=selected?Object.values(selected.inputs).flat()[0]:undefined,candidate=selected?.outputs[0];
 const left:CompareImage|undefined=source?{...source,label:"Source"}:undefined,right:CompareImage|undefined=candidate?{...candidate,label:"Candidate"}:undefined;
 return <div className={styles.workspace}>
  <div className={styles.pickers}><Picker label="Tool" value={toolId} onChange={chooseTool} choices={snapshot.tools.map(t=>({id:t.id,title:t.declaration?.document.tool.name??t.id,description:t.error??`Version ${t.declaration?.document.tool.version??"unknown"}`}))}/><Picker label="Action" value={actionId} onChange={chooseAction} disabled={!declaration} choices={(declaration?.document.actions??[]).map(a=>({id:a.name,title:a.title,description:(a.images??[]).length?"Image → Result":"Parameters → Result"}))}/></div>
  {declaration&&<p className={styles.note}>{declaration.document.tool.purpose}</p>}
  {connection&&<p role="status">{connection}</p>}
  {tool?.error&&<p role="alert">{tool.error}</p>}
  <div className={styles.columns}>
   <form className={styles.panel} onSubmit={event=>{event.preventDefault();void run();}}><h2>Input</h2>
    {action?<ActionForm key={`${toolId}/${actionId}`} action={action} params={params} onChange={setParams} images={images} onImages={(slot,value)=>setImages(values=>({...values,[slot]:value}))} upload={async file=>{const asset=await assets.upload(file);setSavedImages(await assets.list());return asset;}} preview={assets.preview}/>:<p>No supported action is available.</p>}
    {action?.images?.length&&savedImages.length>0?<details><summary>Saved images ({savedImages.length})</summary><div className={styles.saved}>{action.images.map(slot=><div key={slot.slot}><p>Use for {slot.slot}</p>{savedImages.slice(0,12).map((asset,index)=><Button key={asset.id} onClick={()=>setImages(values=>({...values,[slot.slot]:[asset]}))}>Image {index+1} · {asset.width} × {asset.height}</Button>)}</div>)}</div></details>:null}
    {error&&<p role="alert">{error}</p>}<div className={styles.run}><p>{action?.cost.may_spend?"Paid actions are unavailable in this foundation.":"Free action · Inputs are saved locally"}</p><Button type="submit" busy={busy} disabled={!action||action.cost.may_spend||!!working}>Run</Button></div>
   </form>
   <section className={styles.panel} aria-label="Result"><div className={styles.resultHeading}><h2>Result</h2><span role="status">{selected?selected.execution:"Ready"}</span></div>
    {selected?<>{working&&tool?.progress?.message&&<p className={styles.note}>{tool.progress.message}</p>}<ImageCompare left={left} right={right} load={assets.preview}/><div className={styles.resultHeading}><p>Saved attempt · {selected.id.slice(0,8)}</p><Button onClick={event=>setDrawer({opener:event.currentTarget})}>Inspect run</Button></div><p className={styles.note}>Contract evidence: {selected.preservation.contract}. Editing the inputs starts a new draft.</p>{selected.notices.map((notice,index)=><p className={styles.note} key={index}>{notice}</p>)}</>:<div className={styles.empty}><p>Choose your inputs and run an action.</p><p>The result and its saved evidence will appear here.</p></div>}
   </section>
  </div>
  {recent.length>0&&<section className={styles.recent} aria-label="Recent attempts"><h2>Recent attempts</h2>{recent.slice(0,6).map(attempt=><Button key={attempt.id} onClick={()=>setSelected(attempt)}>{attempt.request.action} · {attempt.execution} · {attempt.id.slice(0,8)}</Button>)}</section>}
  <Drawer open={drawer!==null} opener={drawer?.opener??null} onClose={()=>setDrawer(null)} title="Run details" description={selected?`${selected.request.tool} / ${selected.request.action} · ${selected.id}`:""}>{selected&&<Inspector key={selected.id} attempt={selected}/>}</Drawer>
 </div>;
}
