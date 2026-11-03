import styles from "./CommandHandoff.module.css";
import {useEffect,useRef,useState} from "react";
import type {LocalApi} from "../../api/client";
import {handoffClient,type Handoff} from "../../api/handoff";
import {object} from "../actions/model";
import {Button} from "../../components/Button";
type Props={id:string;api:LocalApi;revision?:string|undefined;tool?:{name:string;version:string}|undefined;maySpend?:boolean|undefined};
export function CommandHandoff({id,api,revision="",tool,maySpend=false}:Props){
 const [value,setValue]=useState<Handoff>(),[error,setError]=useState<string>(),[busy,setBusy]=useState(false),[notice,setNotice]=useState<string>();
 const generation=useRef(0);
 useEffect(()=>{generation.current++;let active=true;setValue(undefined);setError(undefined);setNotice(undefined);setBusy(false);void handoffClient(api).read(id).then(value=>{if(active)setValue(value);}).catch(failure=>{if(active)setError(String(failure));});return()=>{active=false;generation.current++;};},[id,api,revision]);
 async function download(){const current=generation.current;setBusy(true);setNotice(undefined);try{const blob=await handoffClient(api).download(id);const url=URL.createObjectURL(blob);const link=document.createElement("a");link.href=url;link.download="wafflery-arguments.json";link.click();setTimeout(()=>URL.revokeObjectURL(url),1000);if(current===generation.current)setNotice("Arguments downloaded. Save the file under the name and directory shown above.");}catch(failure){if(current===generation.current)setNotice(String(failure));}finally{if(current===generation.current)setBusy(false);}}
 const cap=object(value?.invocation?.arguments)?.max_spend_usd;
 return <section className={styles.handoff} aria-label="CLI handoff">
 {tool&&<h3>{tool.name} · {tool.version}</h3>}
 <p>This single-call handoff depends on the configured local store. It is not a portable replay bundle.</p>
 {error?<p role="alert">{error}</p>:!value?<p role="status">Preparing the saved handoff…</p>:<>
 {value.blockers.map((reason,index)=><p role="status" key={index}>{reason}</p>)}
 {maySpend&&<p>This CLI call may spend through the configured provider in a separate session from Wafflery.{typeof cap==="number"?` Saved call cap: ${cap} USD.`:" No explicit call cap was recorded; check the tool's configuration."}</p>}
 {value.command&&<><p>Save {value.arguments_filename} in <code>{value.cwd}</code>, then run:</p><pre aria-label="CLI command">{value.command}</pre>
 <div className={styles.actions}><Button onClick={async()=>{const current=generation.current;try{await navigator.clipboard.writeText(value.command!);if(current===generation.current)setNotice("Copied local command.");}catch{if(current===generation.current)setNotice("Select and copy the command above.");}}}>Copy command</Button>
 <Button busy={busy} onClick={()=>void download()}>Download arguments</Button></div></>}
 {!!value.requirements.length&&<><h3>Requirements</h3><p>Provide these requirements through the tool's configuration and your shell environment. Only their names are recorded.</p><ul>{value.requirements.map(name=><li key={name}>{name}</li>)}</ul></>}
 {!!value.allowed_dirs.length&&<><h3>File access</h3><ul>{value.allowed_dirs.map(path=><li key={path}><code>{path}</code></li>)}</ul></>}
 <p>Use the saved tool version and configuration identity. The command uses your shell environment and starts its own tool session. The host does not read or copy configuration contents or credentials. Keep the original artifact store and owned library files available; a fresh store needs explicit native import or an invocation bundle.</p>
 {value.invocation&&<details><summary>Resolved settings</summary><pre aria-label="Handoff invocation">{JSON.stringify(value.invocation,null,2)}</pre></details>}
 </>}{notice&&<p role="status">{notice}</p>}
 </section>;
}
