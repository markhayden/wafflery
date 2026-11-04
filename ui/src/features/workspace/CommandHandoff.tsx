import styles from "./CommandHandoff.module.css";
import {useEffect,useRef,useState} from "react";
import type {LocalApi} from "../../api/client";
import {handoffClient,type Handoff} from "../../api/handoff";
import {object} from "../actions/model";
import {Button} from "../../components/Button";
import {CodeBlock} from "../../components/CodeBlock";
type Props={id:string;api:LocalApi;revision?:string|undefined;tool?:{name:string;version:string}|undefined;maySpend?:boolean|undefined};

/** Running a saved run again from a terminal, as two steps: download its settings file, run one
 * command. What it needs (keys, folders) follows, and why it can't, when it can't. */
export function CommandHandoff({id,api,revision="",tool,maySpend=false}:Props){
 const [value,setValue]=useState<Handoff>(),[error,setError]=useState<string>(),[busy,setBusy]=useState(false),[notice,setNotice]=useState<string>();
 const generation=useRef(0);
 useEffect(()=>{generation.current++;let active=true;setValue(undefined);setError(undefined);setNotice(undefined);setBusy(false);void handoffClient(api).read(id).then(value=>{if(active)setValue(value);}).catch(failure=>{if(active)setError(String(failure));});return()=>{active=false;generation.current++;};},[id,api,revision]);
 async function download(){const current=generation.current;setBusy(true);setNotice(undefined);try{const blob=await handoffClient(api).download(id);const url=URL.createObjectURL(blob);const link=document.createElement("a");link.href=url;link.download=value?.arguments_filename??"wafflery-arguments.json";link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);if(current===generation.current)setNotice("Settings file downloaded.");}catch(failure){if(current===generation.current)setNotice(String(failure));}finally{if(current===generation.current)setBusy(false);}}
 const cap=object(value?.invocation?.arguments)?.max_spend_usd;
 const name=tool?.name??"the tool";
 return <section className={styles.handoff} aria-label="CLI handoff">
  {error?<p role="alert">{error}</p>:!value?<p role="status">Preparing the command…</p>:value.command?<>
   <div className={styles.run}>
    <span>Paste this into a terminal. The settings are included, so there's nothing to download first.</span>
    <CodeBlock label="CLI command" copyLabel="Copy command" language="shell" value={value.command}/>
   </div>
   <div className={styles.file}>
    <span>For a script, download the settings as a file and pass it with <code>--arguments &lt;file&gt;</code>.</span>
    <Button busy={busy} onClick={()=>void download()}>Download settings file</Button>
   </div>
   {maySpend&&<p className={styles.warning}>This calls the same paid provider and may cost money.{typeof cap==="number"?` It stops at $${cap}, the cap saved with this run.`:" No cap was saved with this run."}</p>}
   {!!value.requirements.length&&<section className={styles.section}><h3>Needs</h3><p>Set these in {name}'s configuration or your shell before running it.</p><ul className={styles.items}>{value.requirements.map(item=><li key={item}><code>{item}</code></li>)}</ul></section>}
   {!!value.allowed_dirs.length&&<section className={styles.section}><h3>Reads files from</h3><ul className={styles.items}>{value.allowed_dirs.map(path=><li key={path}><code>{path}</code></li>)}</ul></section>}
  </>:<div className={styles.blocked} role="status">
   <strong>This run can't be repeated from a terminal yet.</strong>
   {value.blockers.map((reason,index)=><p key={index}>{reason}</p>)}
  </div>}
  {notice&&<p role="status">{notice}</p>}
 </section>;
}
