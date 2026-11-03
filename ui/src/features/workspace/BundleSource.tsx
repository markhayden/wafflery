import styles from "./BundleSource.module.css";
import {useEffect,useMemo,useState} from "react";
import type {LocalApi} from "../../api/client";
import {bundleClient,type BundleRecord} from "../../api/bundles";
import type {Attempt,BundleSource as Source} from "../../api/workspace";
import {object,type Json} from "../actions/model";
import type {UnresolvedInput} from "./bundle-replay";
import {Button} from "../../components/Button";
function identities(result:Json|undefined){const out=object(object(result)?.artifacts)?.out;return Array.isArray(out)?out.map(value=>{const row=object(value),ref=object(row?.artifact);return {port:row?.port,type:ref?.type,identity:ref?.identity};}):[];}
export function BundleSource({api,source,attempt,unresolved,canResolve,resolve}:{api:LocalApi;source:Source;attempt?:Attempt|undefined;unresolved:UnresolvedInput[];canResolve:(name:string)=>boolean;resolve:(name:string)=>void}){
 const client=useMemo(()=>bundleClient(api),[api]);const [record,setRecord]=useState<BundleRecord>(),[error,setError]=useState(""),[expanded,setExpanded]=useState(false);
 useEffect(()=>{let active=true;setRecord(undefined);setError("");void client.read(source.id).then(value=>{if(active)setRecord(value);}).catch(e=>{if(active)setError(String(e));});return()=>{active=false;};},[client,source.id]);
 const original=record?.manifest.steps[source.step];const result=attempt?.request.source_bundle?.id===source.id&&attempt.request.source_bundle.step===source.step?attempt.result:undefined;
 const before=identities(original?.result),after=identities(result);const compared=!!result&&(before.length>0||after.length>0);
 return <section className={styles.source} aria-label="Bundle source">
  <h3>From bundle {source.id.slice(0,8)} · step {source.step+1}</h3>
  <p>This is a new experiment. Inputs and parameters can change; the original invocation and result remain unchanged. Run uses the current tool session and fresh paid estimates.</p>
  {unresolved.length>0&&<><h4>Unresolved imported inputs</h4><p>Select replacements below, then explicitly confirm each port's current input list. Nothing can run or be estimated while an imported input remains unresolved.</p><ul>{unresolved.map(input=><li key={input.name}><strong>{input.name}</strong>{input.reasons.map((reason,index)=><p key={index}>{reason}</p>)}<Button type="button" disabled={!canResolve(input.name)} onClick={()=>resolve(input.name)}>Use current inputs for {input.name}</Button>{!input.required&&<small>An empty selection explicitly omits this optional input in the new draft.</small>}</li>)}</ul></>}
  {compared&&<p role="status">{JSON.stringify(before)===JSON.stringify(after)?"Output artifact identities match the recorded step. Matching identities do not guarantee identical judgments.":"Output artifact identities differ from the recorded step. These are new results; an identity difference alone is not a failure."}</p>}
  {original&&<details onToggle={event=>setExpanded(event.currentTarget.open)}><summary>Original bundled invocation</summary>{expanded&&<pre>{JSON.stringify(original.invocation,null,2)}</pre>}</details>}
  {error&&<p role="status">Original bundle details are unavailable: {error}</p>}
 </section>;
}
