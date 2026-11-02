import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { Picker } from "./Picker";
const meta = { title:"Components/Picker", component:Picker, args:{label:"Action",value:"line",choices:[{id:"line",title:"Line",description:"Image → Location",group:"Find"},{id:"feather",title:"Feather",description:"Mask → Mask",group:"Refine"}],onChange:()=>{}}, render:args=>{const [value,setValue]=useState(args.value);return <Picker {...args} value={value} onChange={setValue}/>;} } satisfies Meta<typeof Picker>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Focused:Story = {};
export const LongList:Story = {args:{choices:Array.from({length:60},(_,index)=>({id:String(index),title:`Action ${index+1}`,description:"A longer description that wraps without displacing the indicator",group:index<30?"Find":"Refine"}))}};
export const Empty:Story = {args:{choices:[],value:""}};
export const Disabled:Story = {args:{disabled:true}};

export const Compact:Story = {args:{compact:true,label:"Tool",value:"sample",choices:[{id:"sample",title:"sample-lab",description:"Version 0.1.0"}]}};
