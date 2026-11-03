import {useState,useEffect,useRef,useLayoutEffect,useId} from "react";
import type {Asset} from "../../api/assets";
import type {BindingIssue} from "../../api/client";
import type {InputBinding} from "../../api/workspace";
import {Button} from "../../components/Button";
import {moveBinding,type Port,type PortBindings} from "./ports";
import type {Schema} from "./model";
import styles from "./ActionForm.module.css";
export type ArtifactChoice={id:string;label:string;reference:Schema;binding?:InputBinding};
type Props={resolveArtifact?:((binding:InputBinding)=>Promise<InputBinding|undefined>)|undefined;ports:Port[];value:PortBindings;onChange:(value:PortBindings)=>void;assets:Asset[];upload:(file:File)=>Promise<Asset>;artifacts?:ArtifactChoice[];issues?:BindingIssue[];disabled?:boolean;onBusy?:((busy:boolean)=>void)|undefined};
export function PortEditor({ports,value,onChange,assets,upload,artifacts=[],issues=[],disabled=false,onBusy,resolveArtifact}:Props){
 const latest=useRef({value,onChange});useLayoutEffect(()=>{latest.current={value,onChange};});
 return <div className={styles.form}>{ports.map(port=><PortRow key={port.name} {...{port,value:value[port.name]??[],assets,upload,artifacts,issues:issues.filter(issue=>issue.port===port.name),disabled,onBusy,resolveArtifact}} onChange={items=>{const current=latest.current;const next={...current.value};if(items.length)next[port.name]=items;else delete next[port.name];current.onChange(next);}} />)}</div>;
}
function PortRow({port,value,onChange,assets,upload,artifacts,issues,disabled,onBusy,resolveArtifact}:Omit<Props,"ports"|"value"|"onChange">&{port:Port;value:InputBinding[];onChange:(items:InputBinding[])=>void}){
 const locationId=useId();
 const [pending,setPending]=useState(false),[error,setError]=useState<string>(),[location,setLocation]=useState("");
 const alive=useRef(true);useEffect(()=>{alive.current=true;return()=>{alive.current=false;};},[]);
 const locked=disabled||pending,full=(!port.many&&value.length>0)||(port.type==="image"&&value.length>=32);
 const choices=(artifacts??[]).filter(item=>item.reference.type===port.type);
 function add(item:InputBinding){onChange([...value,item]);setError(undefined);}
 return <fieldset data-port={port.name} tabIndex={-1} className={styles.form} disabled={locked} aria-busy={pending}>
 <legend>{port.name} · {port.type} · {port.required?"Required":"Optional"}{port.many?" · List":""}</legend>
 {!value.length&&<small>{port.required?"Choose an input to continue.":"No input selected."}</small>}
 <span role="status" aria-label={`${port.name} selection count`}>{value.length} selected</span>
 <ol>{value.map((item,index)=><li key={index}>
 <p>{item.kind==="image"?`Image · ${assets.find(asset=>asset.id===item.asset)?.width??"?"} × ${assets.find(asset=>asset.id===item.asset)?.height??"?"}`:item.kind==="location"?"Shared location":`${item.reference.type} · ${item.reference.identity}`}</p>
 {item.kind==="image"&&<label className={styles.field}>Image coordinates<select value={item.frame?.projection??"flat"} onChange={event=>onChange(value.map((entry,i)=>i===index?{...item,frame:event.target.value==="flat"?{projection:"flat",topology:"bounded"}:{projection:"equirectangular",topology:"wrap"}}:entry))}><option value="flat">Flat image · bounded edges</option><option value="equirectangular">Panorama · horizontal wrap</option></select></label>}
 {item.kind==="location"&&<details><summary>Location data</summary><pre>{JSON.stringify(item.value,null,2)}</pre></details>}
 {item.kind==="artifact"&&<small>Reference selected. Compatibility and availability are checked by the tool on Run.</small>}
 {port.many&&<><Button type="button" disabled={locked||index===0} onClick={()=>onChange(moveBinding(value,index,-1))} aria-label={`Move ${port.name} item ${index+1} up`}>↑</Button><Button type="button" disabled={locked||index===value.length-1} onClick={()=>onChange(moveBinding(value,index,1))} aria-label={`Move ${port.name} item ${index+1} down`}>↓</Button></>}
 <Button type="button" onClick={()=>onChange(value.filter((_,i)=>i!==index))} aria-label={`Remove ${port.name} item ${index+1}`}>Remove</Button>
 {(issues??[]).filter(issue=>issue.index===index).map((issue,i)=><p role="alert" key={i}>{issue.problem}: expected {issue.expected}; got {issue.got}. {issue.remedy}</p>)}
 </li>)}</ol>
 {!full&&<>
 {port.type==="image"&&<><label className={styles.field}>Saved image<select value="" onChange={event=>{if(event.target.value)add({kind:"image",asset:event.target.value,frame:{projection:"flat",topology:"bounded"}});}}><option value="">Choose image…</option>{assets.map(asset=><option key={asset.id} value={asset.id}>{asset.width} × {asset.height} · {asset.id.slice(0,8)}</option>)}</select></label>
 <label className={styles.field}>Import image<input type="file" accept="image/png,image/jpeg" onChange={async event=>{const file=event.target.files?.[0];if(!file)return;event.target.value="";setPending(true);onBusy?.(true);try{const asset=await upload(file);if(!alive.current)return;add({kind:"image",asset:asset.id,frame:{projection:"flat",topology:"bounded"}});}catch(failure){if(alive.current)setError(String(failure));}finally{if(alive.current){setPending(false);onBusy?.(false);}}}}/></label></>}
 {port.type==="location"&&<div className={styles.field}><label htmlFor={locationId}>Shared location JSON</label><textarea id={locationId} value={location} onChange={event=>setLocation(event.target.value)} rows={4}/><Button type="button" onClick={()=>{try{const parsed:unknown=JSON.parse(location);if(!parsed||typeof parsed!=="object"||Array.isArray(parsed))throw Error("Enter a location object.");add({kind:"location",value:parsed as Schema});setLocation("");}catch(failure){setError(String(failure));}}}>Add location</Button></div>}
 <label className={styles.field}>Saved artifact<select value="" onChange={async event=>{const choice=choices.find(item=>item.id===event.target.value);if(!choice)return;const binding=choice.binding?structuredClone(choice.binding):{kind:"artifact" as const,reference:structuredClone(choice.reference)};setPending(true);onBusy?.(true);try{const resolved=resolveArtifact?await resolveArtifact(binding):binding;if(alive.current&&resolved)add(resolved);}catch(failure){if(alive.current)setError(String(failure));}finally{if(alive.current){setPending(false);onBusy?.(false);}}}}><option value="">{choices.length?"Choose artifact…":"No saved artifacts of this type"}</option>{choices.map(item=><option key={item.id} value={item.id}>{item.label}</option>)}</select></label>
 </>}
 {pending&&<p role="status">Importing or resolving input…</p>}{error&&<p role="alert">{error}</p>}
 {(issues??[]).filter(issue=>issue.index===undefined).map((issue,i)=><p role="alert" key={i}>{issue.problem}: expected {issue.expected}; got {issue.got}. {issue.remedy}</p>)}
 </fieldset>;
}
