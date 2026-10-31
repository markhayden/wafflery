import type { Meta,StoryObj } from "@storybook/react-vite";
import {useState} from "react";
import {Drawer} from "./Drawer";
import {Button} from "./Button";
const meta={title:"Components/Drawer",component:Drawer,args:{open:false,onClose:()=>{},title:"Run details",description:"Saved inputs and evidence from this attempt.",children:<><p>A longer inspector with independent scrolling.</p><pre>{Array.from({length:100},(_,i)=>`Step ${i+1}: completed`).join("\n")}</pre><Button>Last action</Button></>},render:function Example(args){const[open,setOpen]=useState(false);const[opener,setOpener]=useState<HTMLElement|null>(null);return <><Button onClick={event=>{setOpener(event.currentTarget);setOpen(true);}}>Inspect run</Button><Drawer {...args} opener={opener} open={open} onClose={()=>setOpen(false)}/></>;}} satisfies Meta<typeof Drawer>;
export default meta;
type Story=StoryObj<typeof meta>;
export const Inspection:Story={};
export const LongTitle:Story={args:{title:"Saved parameters and artifacts from a very long experimental workflow name"}};
