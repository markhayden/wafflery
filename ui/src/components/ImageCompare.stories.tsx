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
/** A mask before and after `close`: the zig-zag's notches fill in. Changes highlights exactly that. */
const draw=(filled:boolean)=>async()=>{const canvas=document.createElement("canvas");canvas.width=480;canvas.height=300;const c=canvas.getContext("2d")!;
 c.fillStyle="#6b5a45";c.fillRect(0,0,480,300);c.strokeStyle="rgba(240,60,230,0.85)";c.lineWidth=24;c.lineJoin="round";c.lineCap="round";
 c.beginPath();[[60,200],[120,90],[180,200],[240,90],[300,200],[360,90],[420,200]].forEach(([x,y],i)=>i?c.lineTo(x!,y!):c.moveTo(x!,y!));c.stroke();
 if(filled){c.fillStyle="rgba(240,60,230,0.85)";for(const x of [120,240,360])c.fill(new Path2D(`M${x-38} 160 L${x} 100 L${x+38} 160 Z`));for(const x of [180,300])c.fill(new Path2D(`M${x-38} 130 L${x} 190 L${x+38} 130 Z`));}
 return await new Promise<Blob>(resolve=>canvas.toBlob(blob=>resolve(blob!),"image/png"));};
const maskImage={width:480,height:300,projection:"flat",topology:"bounded"};
const before={...asset,id:"d".repeat(32),width:480,height:300},after={...asset,id:"e".repeat(32),width:480,height:300};
export const MaskClosed:Story={render:()=><ImageCompare left={{asset:before,image:maskImage,label:"Before · from Rasterize"}} right={{asset:after,image:maskImage,label:"Preview"}}
 load={(value)=>draw(value.id===after.id)()}/>};

/** A panorama whose seam (the left and right edges) is easier to judge with Centre the seam on:
 * a gradient that wraps, a marker at each edge. */
const pano=async()=>{const canvas=document.createElement("canvas");canvas.width=800;canvas.height=400;const c=canvas.getContext("2d")!;
 const g=c.createLinearGradient(0,0,800,0);g.addColorStop(0,"#3b6ea5");g.addColorStop(.5,"#c9a15b");g.addColorStop(1,"#3b6ea5");c.fillStyle=g;c.fillRect(0,0,800,400);
 c.fillStyle="#e84a5f";c.fillRect(0,150,40,100);c.fillRect(760,150,40,100);
 return await new Promise<Blob>(resolve=>canvas.toBlob(blob=>resolve(blob!),"image/png"));};
const panoImage={width:800,height:400,projection:"equirectangular",topology:"wrap"};
const panoAsset={...asset,id:"f".repeat(32),width:800,height:400};
export const Panorama:Story={render:()=><ImageCompare left={{asset:panoAsset,image:panoImage,label:"Source"}} right={{asset:{...panoAsset,id:"0".repeat(32)},image:panoImage,label:"Result"}} load={pano}/>};

/** A tall portrait photo in a short window: Wipe draws it whole, centred, no taller than the window allows. */
const portrait={width:768,height:1376,projection:"flat",topology:"bounded"};
const tall=async()=>{const canvas=document.createElement("canvas");canvas.width=768;canvas.height=1376;const c=canvas.getContext("2d")!;
 c.fillStyle="#6b4f35";c.fillRect(0,0,768,1376);c.fillStyle="#e8d9b5";c.beginPath();c.arc(384,300,120,0,7);c.fill();c.beginPath();c.arc(384,1076,120,0,7);c.fill();
 return await new Promise<Blob>(resolve=>canvas.toBlob(blob=>resolve(blob!),"image/png"));};
const portraitAsset={...asset,id:"1".repeat(32),width:768,height:1376};
export const Portrait:Story={render:()=><ImageCompare left={{asset:portraitAsset,image:portrait,label:"Source"}} right={{asset:{...portraitAsset,id:"2".repeat(32)},image:portrait,label:"Result"}} load={tall}/>};
