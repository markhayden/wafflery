import {useState,useEffect,useRef,useLayoutEffect,useId} from "react";
import {imageHeader} from "../../api/image-header";
import {ImageImportProgress,type ImageImport} from "../../components/ImageImportProgress";
import type {Asset} from "../../api/assets";
import type {BindingIssue} from "../../api/client";
import type {InputBinding} from "../../api/workspace";
import {Button} from "../../components/Button";
import {ActionRow} from "../../components/ActionRow";
import {ImageThumb,ImageThumbGrid,ImageTile} from "../../components/ImageThumb";
import {moveBinding,type Port,type PortBindings} from "./ports";
import type {Json,Schema} from "./model";
import type {SelectedImage} from "./selection";
import {InfoTip} from "../../components/InfoTip";
import {CodeBlock} from "../../components/CodeBlock";
import {humanize,locationHelp,locationLabel} from "./help";
import {RectangleDrawer} from "./RectangleDrawer";
import {StrokeDrawer} from "./StrokeDrawer";
import styles from "./ActionForm.module.css";
export type ArtifactChoice={id:string;label:string;reference:Schema;binding?:InputBinding;disabled?:boolean};
type Props={drawOn?:SelectedImage|undefined;preview?:((asset:Asset)=>Promise<Blob>)|undefined;resolveArtifact?:((binding:InputBinding)=>Promise<InputBinding|undefined>)|undefined;ports:Port[];value:PortBindings;onChange:(value:PortBindings)=>void;assets:Asset[];upload:(file:File)=>Promise<Asset>;artifacts?:ArtifactChoice[];issues?:BindingIssue[];disabled?:boolean;onBusy?:((busy:boolean)=>void)|undefined};
export function PortEditor({ports,value,onChange,assets,upload,artifacts=[],issues=[],disabled=false,onBusy,resolveArtifact,drawOn,preview}:Props){
 const latest=useRef({value,onChange});useLayoutEffect(()=>{latest.current={value,onChange};});
 return <div className={styles.form}>{ports.map(port=><PortRow key={port.name} {...{port,value:value[port.name]??[],assets,upload,artifacts,issues:issues.filter(issue=>issue.port===port.name),disabled,onBusy,resolveArtifact,drawOn,preview}} onChange={items=>{const current=latest.current;const next={...current.value};if(items.length)next[port.name]=items;else delete next[port.name];current.onChange(next);}} />)}</div>;
}
function PortRow({port,value,onChange,assets,upload,artifacts,issues,disabled,onBusy,resolveArtifact,drawOn,preview}:Omit<Props,"ports"|"value"|"onChange">&{port:Port;value:InputBinding[];onChange:(items:InputBinding[])=>void}){
 const locationId=useId();
 const [imageImport,setImageImport]=useState<ImageImport>();
 const [pending,setPending]=useState(false),[error,setError]=useState<string>(),[location,setLocation]=useState("");
 const fileInput=useRef<HTMLInputElement>(null);
 const alive=useRef(true);useEffect(()=>{alive.current=true;return()=>{alive.current=false;};},[]);
 const locked=disabled||pending,full=(!port.many&&value.length>0)||(port.type==="image"&&value.length>=32);
 const choices=(artifacts??[]).filter(item=>item.reference.type===port.type);
 const label=port.type==="location"?locationLabel(port.name,port.kinds):humanize(port.name);
 const help=port.type==="location"?locationHelp(port.kinds):`An input of type ${port.type}${port.many?"; you can add several, in order":""}.`;
 // The shape drawn on the image, when there is a tool for one of the kinds the slot accepts.
 const drawKind=port.type!=="location"?undefined:(port.kinds??[]).includes("bbox")?"bbox":(port.kinds??[]).includes("stroke")?"stroke":undefined;
 const drawsBox=!!drawKind;
 // Locked until it can be done: an area drawn on an image waits for the image, as one dimmed line.
 if(drawsBox&&!drawOn&&!value.length)return <fieldset data-port={port.name} tabIndex={-1} className={`${styles.form} ${styles.locked}`}>
  <legend className={styles.legend}><span>{label}</span><span className={styles.tag}>{port.required?"Required":"Optional"}</span><InfoTip label={label}>{help}</InfoTip></legend>
  <small>Choose a source image above first; you then {drawKind==="stroke"?"paint over the damage":"draw the area"} on it.</small>
  <span role="status" className={styles.hidden} aria-label={`${port.name} selection count`}>0 selected</span>
 </fieldset>;
 function add(item:InputBinding){onChange([...value,item]);setError(undefined);}
 return <fieldset data-port={port.name} tabIndex={-1} className={styles.form} disabled={locked} aria-busy={pending}>
 <legend className={styles.legend}><span>{label}</span><span className={styles.tag}>{port.required?"Required":"Optional"}{port.many?" · List":""}</span><InfoTip label={label}>{help}</InfoTip></legend>
 {!value.length&&port.type!=="image"&&<small>{port.type==="location"?(drawsBox?(drawOn?(drawKind==="stroke"?"Not set yet. Paint over it on your image below.":"Not set yet. Draw it on your image below."):"Choose the image above first, then draw the area on it."):"Not set yet. Paste a location below."):port.required?"Choose an input to continue.":"No input selected."}</small>}
 <span role="status" className={port.many?undefined:styles.hidden} aria-label={`${port.name} selection count`}>{value.length} selected</span>
 {value.length>0&&<ol className={port.many?undefined:styles.single}>{value.map((item,index)=>{const asset=item.kind==="image"?assets.find(candidate=>candidate.id===item.asset):undefined;return <li key={index}>
 {item.kind==="image"?<div className={styles.chosen}><ImageTile asset={asset} preview={preview}/><div className={styles.chosenText}><strong>{asset?`${asset.width} × ${asset.height}`:"Image unavailable"}</strong>{port.many&&<small>Image {index+1} of {value.length}</small>}</div></div>:<p>{item.kind==="location"?describeLocation(item.value):(choices.find(choice=>choice.reference.identity===item.reference.identity)?.label??`${humanize(String(item.reference.type).replace(/^[a-z]+:/,""))} from an earlier run`)}</p>}
 {item.kind==="image"&&<label className={styles.field}>Image coordinates<select value={item.frame?.projection??"flat"} onChange={event=>onChange(value.map((entry,i)=>i===index?{...item,frame:event.target.value==="flat"?{projection:"flat",topology:"bounded"}:{projection:"equirectangular",topology:"wrap"}}:entry))}><option value="flat">Flat image · bounded edges</option><option value="equirectangular">Panorama · horizontal wrap</option></select></label>}
 {item.kind==="location"&&<details className={styles.advanced}><summary>Location data</summary><CodeBlock label="Location data" value={item.value}/></details>}
 {item.kind==="artifact"&&<small>From an earlier run. The tool checks it can use it when you run.</small>}
 <ActionRow>{port.many&&<><Button type="button" disabled={locked||index===0} onClick={()=>onChange(moveBinding(value,index,-1))} aria-label={`Move ${port.name} item ${index+1} up`}>↑</Button><Button type="button" disabled={locked||index===value.length-1} onClick={()=>onChange(moveBinding(value,index,1))} aria-label={`Move ${port.name} item ${index+1} down`}>↓</Button></>}<Button type="button" variant="destructive" onClick={()=>onChange(value.filter((_,i)=>i!==index))} aria-label={`Remove ${port.name} item ${index+1}`}>Remove</Button></ActionRow>
 {(issues??[]).filter(issue=>issue.index===index).map((issue,i)=><p role="alert" key={i}>{issue.problem}: expected {issue.expected}; got {issue.got}. {issue.remedy}</p>)}
 </li>;})}</ol>}
 {!full&&<>
 {port.type==="image"&&<div className={styles.imageChoice}>
  <div className={styles.ownImage}><Button type="button" disabled={locked} onClick={()=>fileInput.current?.click()}>Use your own image</Button><small>PNG or JPEG · up to 128 MiB</small></div>
  <input ref={fileInput} hidden type="file" accept="image/png,image/jpeg" aria-label={`Use your own image as ${label.toLowerCase()}`} onChange={async event=>{const file=event.target.files?.[0];if(!file)return;event.target.value="";setPending(true);setError(undefined);setImageImport({name:file.name});onBusy?.(true);try{if(file.size>128*1024*1024)throw Error("Images must be no larger than 128 MiB.");const dimensions=await imageHeader(file);if(!alive.current)return;setImageImport({name:file.name,dimensions});const asset=await upload(file);if(!alive.current)return;add({kind:"image",asset:asset.id,frame:{projection:"flat",topology:"bounded"}});}catch(failure){if(alive.current)setError(String(failure));}finally{if(alive.current){setPending(false);setImageImport(undefined);onBusy?.(false);}}}}/>
  {assets.length>0&&<details className={styles.advanced}><summary>Recent images</summary><ImageThumbGrid>{assets.slice(0,4).map(asset=><ImageThumb key={asset.id} asset={asset} preview={preview} disabled={locked}
   label={`Use recent image ${asset.width} × ${asset.height} as ${label.toLowerCase()}`} onPick={()=>add({kind:"image",asset:asset.id,frame:{projection:"flat",topology:"bounded"}})}/>)}</ImageThumbGrid></details>}
 </div>}
 {drawKind==="bbox"&&drawOn&&preview&&<RectangleDrawer image={drawOn} preview={preview} disabled={locked} onUse={(location:Json)=>add({kind:"location",value:location as Schema})}/>}
 {drawKind==="stroke"&&drawOn&&preview&&<StrokeDrawer image={drawOn} preview={preview} disabled={locked} onUse={(location:Json)=>add({kind:"location",value:location as Schema})}/>}
 {port.type==="location"&&(()=>{const paste=<div className={styles.field}><label htmlFor={locationId}>Location JSON</label><textarea id={locationId} value={location} onChange={event=>setLocation(event.target.value)} rows={4}/><Button type="button" disabled={!location.trim()} onClick={()=>{try{const parsed:unknown=JSON.parse(location);if(!parsed||typeof parsed!=="object"||Array.isArray(parsed))throw Error("Enter a location object.");add({kind:"location",value:parsed as Schema});setLocation("");}catch(failure){setError(failure instanceof SyntaxError?"That isn't valid JSON. Paste a whole location object, from { to }.":String(failure instanceof Error?failure.message:failure));}}}>Add location</Button></div>;
  return drawsBox?<details className={styles.advanced}><summary>Paste a location as JSON instead</summary>{paste}</details>:paste;})()}
 {choices.length>0&&<label className={styles.field}>Saved artifact<select value="" onChange={async event=>{const choice=choices.find(item=>item.id===event.target.value);if(!choice)return;const binding=choice.binding?structuredClone(choice.binding):{kind:"artifact" as const,reference:structuredClone(choice.reference)};setPending(true);onBusy?.(true);try{const resolved=resolveArtifact?await resolveArtifact(binding):binding;if(alive.current&&resolved)add(resolved);}catch(failure){if(alive.current)setError(String(failure));}finally{if(alive.current){setPending(false);setImageImport(undefined);onBusy?.(false);}}}}><option value="">{choices.length?"Choose artifact…":"No saved artifacts of this type"}</option>{choices.map(item=><option key={item.id} value={item.id} disabled={item.disabled}>{item.label}</option>)}</select></label>}
 </>}
 {pending&&(imageImport?<ImageImportProgress image={imageImport}/>:<p role="status">Resolving input…</p>)}{error&&<p role="alert">{error}</p>}
 {(issues??[]).filter(issue=>issue.index===undefined).map((issue,i)=><p role="alert" key={i}>{issue.problem}: expected {issue.expected}; got {issue.got}. {issue.remedy}</p>)}
 </fieldset>;
}

/** A short, readable account of a location: "Rectangle · 120 × 80 px at 34, 50". */
export function describeLocation(value:Schema):string{
 const image=value.image as {width?:number;height?:number}|undefined;
 const part=Array.isArray(value.parts)?value.parts[0] as {bbox?:number[]}|undefined:undefined;
 if(value.kind==="bbox"&&part?.bbox?.length===4&&image?.width&&image.height){
  const [x0,y0,x1,y1]=part.bbox as [number,number,number,number];
  const left=Math.round(x0*image.width),top=Math.round(y0*image.height);
  return `Rectangle · ${Math.round(x1*image.width)-left} × ${Math.round(y1*image.height)-top} px at ${left}, ${top}`;
 }
 const strokes=Array.isArray(value.parts)?(value.parts as {path?:unknown[];radius_px?:number}[]):[];
 if(value.kind==="stroke"&&strokes.length)return `${strokes.length===1?"Brush stroke":`${strokes.length} brush strokes`} · ${strokes.reduce((sum,part)=>sum+(part.path?.length??0),0)} points · ${Math.max(...strokes.map(part=>part.radius_px??0))*2} px brush`;
 return typeof value.kind==="string"?`Location · ${humanize(value.kind)}`:"Location";
}
