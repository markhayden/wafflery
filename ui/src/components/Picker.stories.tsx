import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { toolChoice } from "../features/workspace/tool-presentation";
import type {RegisteredTool} from "../api/workspace";
import { Picker } from "./Picker";
const meta = { title:"Components/Picker", component:Picker, args:{label:"Action",value:"line",choices:[{id:"line",title:"Line",description:"Image → Location",group:"Find"},{id:"feather",title:"Feather",description:"Mask → Mask",group:"Refine"}],onChange:()=>{}}, render:args=>{const [value,setValue]=useState(args.value);return <Picker {...args} value={value} onChange={setValue}/>;} } satisfies Meta<typeof Picker>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Focused:Story = {};
export const LongList:Story = {args:{choices:Array.from({length:60},(_,index)=>({id:String(index),title:`Action ${index+1}`,description:"A longer description that wraps without displacing the indicator",group:index<30?"Find":"Refine"}))}};
export const Empty:Story = {args:{choices:[],value:""}};
export const Disabled:Story = {args:{disabled:true}};

export const Compact:Story = {args:{compact:true,wide:true,label:"Tool",value:"sample",choices:[{id:"sample",title:"sample-lab",meta:"Version 0.1.0",description:"A local image pipeline tool."}]}};

const declared=(name:string,version:string,purpose:string,error:string|null=null)=>toolChoice({id:name,error,
  declaration:{document:{tool:{name,version,purpose}}}} as RegisteredTool);
export const Tools:Story = {args:{compact:true,wide:true,label:"Tool",value:"rascal",choices:[
 declared("rascal","0.1.1","Contract purpose."),declared("squint","0.1.0 (6b9bfa327)","Contract purpose."),
 declared("another-tool","0.2.0","Explores a locally installed tool using its declared purpose."),
 declared("offline-tool","0.1.0","This purpose loses to the error.","Cannot start this tool. Check its installation."),
]}};
export const LongToolVersion:Story = {args:{...Tools.args,choices:[declared("rascal","0.1.0 (6b9bfa327-with-a-very-long-build-identifier)","Contract purpose."),
 declared("another-tool-with-a-long-name","0.2.0 (another-long-build-identifier)","An unknown tool’s declared purpose still wraps cleanly.")]}};
