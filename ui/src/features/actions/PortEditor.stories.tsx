import type {Meta,StoryObj} from "@storybook/react-vite";
import {useState} from "react";
import {PortEditor} from "./PortEditor";
const reference={type:"study:Mask",schema_version:1,identity:"sha256:example",resource:"study://artifact/example",frame:"flat",on:null};
const meta={title:"Workspace/PortEditor",component:PortEditor,args:{ports:[{name:"image",type:"image",required:true},{name:"guides",type:"location",many:true},{name:"masks",type:"study:Mask",required:true,many:true}],value:{},onChange:()=>{},assets:[],upload:async()=>{throw Error("Story import disabled");},artifacts:[{id:"mask",label:"Saved mask · preview unavailable",reference}]},render:args=>{const [value,setValue]=useState(args.value);return <PortEditor {...args} value={value} onChange={setValue}/>;}} satisfies Meta<typeof PortEditor>;
export default meta;type Story=StoryObj<typeof meta>;
export const Empty:Story={};
export const Refused:Story={args:{value:{masks:[{kind:"artifact",reference},{kind:"artifact",reference}]},issues:[{port:"masks",index:1,problem:"unresolved",expected:"study:Mask",got:"missing",remedy:"Import the saved bundle."}]}};
export const Resolving:Story={args:{disabled:true,value:{masks:[{kind:"artifact",reference}]}}};
export const FilledShapes:Story={args:{ports:[{name:"image",type:"image",required:true},{name:"location",type:"location",required:true},{name:"mask",type:"study:Mask"}],assets:[{id:"a".repeat(32),sha256:"b".repeat(64),width:2048,height:1024,size:100,media_type:"image/png"}],value:{image:[{kind:"image",asset:"a".repeat(32)}],location:[{kind:"location",value:{kind:"point",x:10,y:20}}],mask:[{kind:"artifact",reference}]}}};
export const DeferredImport:Story={render:args=>{
 const [value,setValue]=useState(args.value),[finish,setFinish]=useState<(()=>void)>();
 return <><PortEditor {...args} value={value} onChange={setValue} upload={()=>new Promise(resolve=>setFinish(()=>()=>resolve({id:"a".repeat(32),sha256:"b".repeat(64),width:16,height:8,size:10,media_type:"image/png"})))}/><button type="button" disabled={!finish} onClick={()=>finish?.()}>Finish import</button><pre data-testid="bindings">{JSON.stringify(value)}</pre></>;
}};
export const ImportFailure:Story={args:{upload:async()=>{throw Error("Image could not be read.");}}};
export const SavedChain:Story={args:{ports:[{name:"mask",type:"study:Mask",required:true,many:true}],value:{mask:[{kind:"artifact",reference,origin:{attempt:"upstream",port:"mask",occurrence:0}},{kind:"artifact",reference,origin:{attempt:"upstream",port:"mask",occurrence:1}}]},artifacts:[{id:"first",label:"Upstream / mask / item 1",reference,binding:{kind:"artifact",reference,origin:{attempt:"upstream",port:"mask",occurrence:0}}}]}};
