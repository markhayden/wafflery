import type {Meta,StoryObj} from "@storybook/react-vite";
import {ImageCompare} from "./ImageCompare";
import sample from "../../tests/fixtures/sample.png?url";
const asset={id:"a".repeat(32),sha256:"b".repeat(64),size:10063,width:160,height:100,media_type:"image/png" as const};
const image={width:160,height:100,projection:"flat",topology:"bounded"};
const meta={title:"Components/ImageCompare",component:ImageCompare,args:{left:{asset,image,label:"Source"},right:{asset:{...asset,id:"c".repeat(32)},image,label:"Candidate"},load:async()=> (await fetch(sample)).blob()}} satisfies Meta<typeof ImageCompare>;
export default meta;
type Story=StoryObj<typeof meta>;
export const Wipe:Story={};
export const Incompatible:Story={args:{right:{asset,image:{...image,width:320},label:"Different frame"}}};
export const Missing:Story={args:{right:undefined}};
export const Unavailable:Story={args:{load:async()=>{throw new Error("Missing bytes");}}};
export const LongLabels:Story={args:{left:{asset,image,label:"A very long source image name from a saved experiment"}}};

export const Loading:Story = {args:{load:()=>new Promise(()=>{})}};
