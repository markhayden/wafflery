import {ChainReplays,ChainSetup} from "./ChainReplays";
import type {ReplayStatus} from "../../api/chain-replays";
import type {BundleUpload} from "../../api/bundle-imports";
import {ChainDependencies,ChainSelection} from "./BundleChain";
import {BundleImports} from "./BundleImports";
import {useEffect,useMemo,useRef,useState,type ReactNode} from "react";
import {bundleClient,cliBundleTool,type BundleRecord,type BundleSummary} from "../../api/bundles";
import type {LocalApi} from "../../api/client";
import type {Attempt} from "../../api/workspace";
import {object} from "../actions/model";
import {Button} from "../../components/Button";
import styles from "./InvocationBundles.module.css";
function choices(attempt?:Attempt):string[]{
 const action=attempt?.declaration.document?.actions.find(a=>a.name===attempt.request.action);
 const args=object(object(object(attempt?.result)?.invocation)?.arguments);
 if(!action||!args)return[];
 const inputs=action.kind==="operation"?(action.ports?.in??[]).map(p=>({name:p.name,type:p.type,many:p.many})):(action.images??[]).map(p=>({name:p.slot,type:"image",many:p.many}));
 return inputs.filter(p=>{
  const v=args[p.name];if(v===undefined)return false;const values=p.many&&Array.isArray(v)?v:[v];
  return values.length>0&&(p.type!=="location"||values.every(v=>typeof object(v)?.resource==="string"));
 }).map(p=>p.name);
}
const fileSize=(bytes:number)=>bytes<1024*1024?`${(bytes/1024).toFixed(1)} KB`:`${(bytes/1024/1024).toFixed(2)} MiB`;
export function InvocationBundles({api,attempt,onOpenImported,onOpenChain,activeReplayId}:{api:LocalApi;attempt?:Attempt;onOpenImported?:((upload:BundleUpload)=>Promise<void>)|undefined;onOpenChain?:((status:ReplayStatus)=>Promise<void>)|undefined;activeReplayId?:string|undefined}){
 const client=useMemo(()=>bundleClient(api),[api]);const generation=useRef(0);
 const [replayOpen,setReplayOpen]=useState(!!activeReplayId),[replayMounted,setReplayMounted]=useState(!!activeReplayId),[chosenReplay,setChosenReplay]=useState(activeReplayId);
 function started(status:ReplayStatus){setChosenReplay(status.id);setReplayMounted(true);setReplayOpen(true);}
 const [importOpen,setImportOpen]=useState(false);const [chainOpen,setChainOpen]=useState(false);
 const [items,setItems]=useState<BundleSummary[]>([]),[cursor,setCursor]=useState<number|null>(null),[selected,setSelected]=useState<BundleRecord>(),[busy,setBusy]=useState(true),[notice,setNotice]=useState(""),[omit,setOmit]=useState<string[]>([]);
 useEffect(()=>{
  const current=++generation.current;setBusy(true);setSelected(undefined);setItems([]);setOmit([]);setNotice("");
  void client.list(attempt?.id).then(page=>{if(current===generation.current){setItems(page.items);setCursor(page.cursor);}}).catch(e=>{if(current===generation.current)setNotice(String(e));}).finally(()=>{if(current===generation.current)setBusy(false);});
  return()=>{generation.current++;};
 },[client,attempt?.id]);
 async function run(operation:()=>Promise<void>){const current=generation.current;setBusy(true);setNotice("");try{await operation();}catch(e){if(current===generation.current)setNotice(e instanceof Error?e.message:String(e));}finally{if(current===generation.current)setBusy(false);}}
 async function create(){if(!attempt||busy)return;const current=generation.current;setSelected(undefined);await run(async()=>{
  const record=await client.create(attempt.id,Object.fromEntries(omit.map(name=>[name,"Left out explicitly when this bundle was saved."])));
  if(current!==generation.current)return;setSelected(record);const page=await client.list(attempt.id);if(current===generation.current){setItems(page.items);setCursor(page.cursor);}
 });}
 async function open(id:string){if(busy)return;const current=generation.current;setSelected(undefined);await run(async()=>{const record=await client.read(id);if(current===generation.current)setSelected(record);});}
 async function more(){if(busy||cursor===null)return;const current=generation.current;await run(async()=>{const page=await client.list(attempt?.id,cursor);if(current===generation.current){setItems(previous=>[...previous,...page.items]);setCursor(page.cursor);}});}
 async function download(){if(!selected||busy)return;const current=generation.current;const id=selected.id;await run(async()=>{
  const blob=await client.download(id);if(current!==generation.current)return;
  const url=URL.createObjectURL(blob);const link=document.createElement("a");link.href=url;link.download="wafflery-invocation.tar";link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);setNotice("Saved invocation bundle downloaded.");
 });}
 const omittable=choices(attempt);const ready=!!attempt?.result&&object(object(attempt.result)?.invocation)!==undefined&&attempt.preservation.contract!=="pending"&&attempt.preservation.native!=="preserving";
 return <section className={styles.bundles} aria-label="Invocation bundles">
  <p>Invocation bundles carry the recorded call and its exact preserved inputs. They are separate from evidence archives. Saving or downloading one never runs a tool.</p>
  <details onToggle={event=>setImportOpen(event.currentTarget.open)}><summary>Import an invocation bundle</summary>{importOpen&&<BundleImports api={api} onOpen={onOpenImported} onChainStarted={started}/>}</details>
  <details onToggle={event=>{if(event.currentTarget.open)setChainOpen(true);}}><summary>Build a bundle from saved runs</summary>{chainOpen&&<ChainSelection key={attempt?.id??"library"} api={api} disabled={busy} onBusy={setBusy} onSaved={async record=>{const current=generation.current;setSelected(record);const page=await client.list(attempt?.id);if(current===generation.current){setItems(page.items);setCursor(page.cursor);}}}/>}</details>
  <details open={replayOpen} onToggle={event=>{setReplayOpen(event.currentTarget.open);if(event.currentTarget.open)setReplayMounted(true);}}><summary>Chain reviews</summary>{replayMounted&&<ChainReplays api={api} activeId={chosenReplay} onOpen={onOpenChain}/>}</details>
  {attempt&&<fieldset disabled={busy}><legend>New bundle from this saved run</legend>
   <p>Inputs are included by default. Omitting a whole slot or port makes the bundle workspace-dependent. Shared location values remain in the recorded invocation.</p>
   {omittable.map(name=><label key={name}><input type="checkbox" checked={!omit.includes(name)} onChange={e=>setOmit(previous=>e.target.checked?previous.filter(v=>v!==name):[...previous,name])}/> Include {name} inputs</label>)}
   <Button disabled={!ready} onClick={()=>void create()}>Save bundle for review</Button>
   {!ready&&<p>Wait for a resolved invocation and input preservation to finish. Historical or unresolved runs retain their evidence archive.</p>}
  </fieldset>}
  {busy&&<p role="status">Reading and verifying saved bundle data…</p>}
  {!!items.length&&<section aria-label="Saved bundle list"><h3>{attempt?"Bundles linked to this run":"Saved bundles"}</h3><ul>{items.map(item=><li key={item.id}><Button disabled={busy} aria-pressed={selected?.id===item.id} onClick={()=>void open(item.id)}>Review {item.tool.name} / {item.action} · {item.id.slice(0,8)}</Button><span>{item.workspace_dependent?"Workspace-dependent":"Inputs included"} · {fileSize(item.size)}</span></li>)}</ul></section>}
  {!busy&&items.length===0&&<p>No invocation bundles are linked{attempt?" to this run":" in this library"}.</p>}
  {cursor!==null&&<Button disabled={busy} onClick={()=>void more()}>Load older bundles</Button>}
  {selected&&<BundleReview bundle={selected} busy={busy} onDownload={()=>void download()}>{selected.manifest.steps.length>1&&<ChainSetup api={api} record={selected} onStarted={started}/>}</BundleReview>}
  {notice&&<p role="status">{notice}</p>}
 </section>;
}
export function BundleReview({bundle,busy=false,onDownload,children}:{bundle:BundleRecord;busy?:boolean;onDownload:()=>void;children?:ReactNode}){
 const cli=cliBundleTool(bundle.manifest,bundle.chain);const requirements=[...new Set(bundle.manifest.steps.flatMap(s=>s.invocation.requirements))];
 return <section className={styles.review} aria-label="Saved bundle review">
  <header><h3>{bundle.workspace_dependent?"Workspace-dependent bundle":"Inputs included"}</h3><p>Saved bundle · {bundle.id.slice(0,12)} · {fileSize(bundle.size)}</p></header>
  <p>{bundle.workspace_dependent?"Some inputs still need the original store or an explicitly supplied replacement. This bundle cannot be replayed as-is.":"Original input bytes and supported native exports are included. Install/configure the named tool and provide its requirements separately. Model judgments may change on replay."}</p>
  <ChainDependencies bundle={bundle}/>{children}
  <section aria-label="Bundle files"><h4>Included files</h4><ul><li><code>invocation.json</code> · Original invocation and result</li>{bundle.members.map(member=><li key={member.name}><code title={member.name} aria-label={member.name}>{member.name.replace(/([a-f0-9]{12})[a-f0-9]{52}/g,"$1…")}</code><span>{fileSize(member.bytes)} · {member.sha256.slice(0,12)}</span></li>)}</ul></section>
  <section><h4>Requirements</h4>{requirements.length?<ul>{requirements.map(name=><li key={name}>{name}</li>)}</ul>:<p>No additional requirements were recorded.</p>}<p>Credentials and tool configuration contents are not included. Replay uses the current tool session and requires fresh estimates and caps for paid actions.</p></section>
  {bundle.manifest.steps.map((step,index)=><section key={index} aria-label={`Bundle step ${index+1}`}><h4>{index+1}. {step.tool.name} / {step.invocation.action}</h4><p>Recorded tool version {step.tool.version}</p>
   {!!step.artifacts.length&&<><h4>Native input copies</h4><ul>{step.artifacts.map((entry,index)=><li key={index}>{entry.port} · {String(entry.artifact.type??"Artifact")} · <code>{String(entry.artifact.identity??"")}</code></li>)}</ul></>}
   {!!step.reference_only.length&&<><h4>Omitted inputs</h4><ul>{step.reference_only.map((entry,index)=><li key={index}>{entry.port??entry.slot}: {entry.why}</li>)}</ul></>}
   {!!step.unportable.length&&<><h4>Unresolved native inputs</h4><ul>{step.unportable.map((entry,index)=><li key={index}>{entry.port}: {entry.reason}</li>)}</ul></>}
   <details><summary>Recorded invocation</summary><pre>{JSON.stringify(step.invocation,null,2)}</pre></details>
  </section>)}
  {cli&&<section><h4>Use with the tool CLI</h4><p>After downloading, replace both example paths with your saved file and its containing folder. Tool setup and any paid use remain separate.</p><pre aria-label="Bundle CLI example">{`'${cli}' call --bundle '/absolute/path/to/wafflery-invocation.tar' --allow-dir '/absolute/path/to'`}</pre></section>}
  {!cli&&!bundle.workspace_dependent&&new Set(bundle.manifest.steps.map(s=>s.tool.name)).size>1&&<p>This bundle needs host replay because it uses more than one tool.</p>}
  <Button busy={busy} onClick={onDownload}>Download saved invocation bundle</Button>
  <p>SHA-256: <code>{bundle.sha256}</code>. The saved archive is verified before each download.</p>
 </section>;
}
