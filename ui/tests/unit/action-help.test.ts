import {expect,test} from "bun:test";
import {humanize,imageHelp,imageLabel,locationHelp,locationLabel,paramHelp,paramLabel} from "../../src/features/actions/help";
import {bboxLocation,clampBox} from "../../src/features/actions/RectangleDrawer";
import {describeLocation} from "../../src/features/actions/PortEditor";

test("names read as words, with units spelled out",()=>{
 expect(humanize("max_spend_usd")).toBe("Max spend (USD)");
 expect(humanize("grow-px")).toBe("Grow (px)");
 expect(humanize("strict")).toBe("Strict");
 expect(humanize("usd")).toBe("Usd");
});
test("images and locations get plain labels and help for newcomers",()=>{
 expect(imageLabel({slot:"image",role:"source"})).toBe("Source image");
 expect(imageLabel({slot:"candidate",role:"candidate"})).toBe("Candidate image");
 expect(imageLabel({slot:"before",role:"reference"})).toBe("Before (reference image)");
 expect(imageHelp({slot:"image",role:"source"})).toContain("never changed");
 expect(locationLabel("location",["bbox"])).toBe("Area (a rectangle)");
 expect(locationLabel("allowed",["mask","bbox"])).toBe("Allowed (a mask image or a rectangle)");
 expect(locationHelp(["bbox"])).toContain("drag a rectangle");
 expect(locationHelp(["line"])).toContain("Paste it as JSON");
});
test("settings use the schema's title, else the key, and explain what they accept",()=>{
 expect(paramLabel("max_spend_usd",{})).toBe("Max spend (USD)");
 expect(paramLabel("x",{title:"Exact title"})).toBe("Exact title");
 expect(paramHelp({description:"Cap.",type:"number",exclusiveMinimum:0},false)).toBe("Cap. A number, above 0. Optional: leave it empty to leave it out.");
 expect(paramHelp({type:"number",default:0.5},false)).toBe("A number. Default: 0.5. Optional: leave it empty to use the tool's default.");
 expect(paramHelp({type:"string",enum:["flat","equirectangular"]},true)).toBe("Choices: flat, equirectangular.");
 expect(paramHelp({type:"boolean",default:false},false)).toContain("Default: false.");
});
test("a drawn rectangle becomes the contract's bbox location on the original image",()=>{
 const image={id:"a".repeat(32),sha256:"b".repeat(64),size:1,width:1440,height:1000,media_type:"image/png" as const};
 const box=clampBox({left:360.4,top:299.6,width:648,height:300},1440,1000);
 expect(box).toEqual({left:360,top:300,width:648,height:300});
 const location=bboxLocation(image,box) as {image:unknown;kind:string;parts:{bbox:number[]}[];straddles_wrap:boolean};
 expect(location.image).toEqual({file_sha256:"b".repeat(64),width:1440,height:1000,projection:"flat",topology:"bounded"});
 expect(location.kind).toBe("bbox");expect(location.straddles_wrap).toBe(false);
 expect(location.parts[0]!.bbox).toEqual([0.25,0.3,0.7,0.6]);
 expect(describeLocation(location as never)).toBe("Rectangle · 648 × 300 px at 360, 300");
 expect(clampBox({left:-5,top:990,width:5000,height:50},1440,1000)).toEqual({left:0,top:990,width:1440,height:10});
});
