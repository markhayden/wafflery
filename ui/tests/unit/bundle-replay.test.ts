import {test,expect} from "bun:test";
import {importedDraft} from "../../src/features/workspace/bundle-replay";
import type {Action,Declaration} from "../../src/features/actions/model";
import type {BundleUpload} from "../../src/api/bundle-imports";
const action:Action={kind:"operation",name:"shape",title:"Shape",purpose:"Test",params_schema:{type:"object"},cost:{may_spend:false},readiness:{state:"ready",requirements:[]},ports:{in:[{name:"pictures",type:"image",required:true,many:true},{name:"tokens",type:"sample:Token",many:true}],out:[]}};
const declaration:Declaration={document:{contract:"0.6.0",tool:{name:"sample",version:"2.0.0",purpose:"Test"},actions:[action]},revision:"current",generation:1,schemas:{}};
const image={id:"b".repeat(32),sha256:"c".repeat(64),width:160,height:100,size:1024,media_type:"image/png" as const};
function review():BundleUpload{return {id:"d".repeat(32),record:{id:"a".repeat(64),sha256:"a".repeat(64),size:2048,manifest:{bundle_version:1,steps:[{contract:"0.6.0",tool:{name:"sample",version:"1.0.0"},invocation:{action:"shape",arguments:{},effective:{},requirements:[]},result:{},inputs:[],artifacts:[],reference_only:[],unportable:[]}]},members:[],workspace_dependent:false},import:{phase:"completed",bundle:"a".repeat(64),step:0,tool:"installed",action:"shape",params:{amount:0.75},inputs:[{name:"pictures",index:0,status:"completed",binding:{kind:"image",asset:image.id,frame:{projection:"equirectangular",topology:"wrap"}},reason:null},{name:"pictures",index:1,status:"completed",binding:{kind:"image",asset:image.id},reason:null},{name:"tokens",index:0,status:"unresolved",binding:null,reason:"No export"}],error:null}};}
test("imported operation inputs keep duplicate order and unresolved optional ports block replay until acknowledged",async()=>{
 const upload=review(),original=structuredClone(upload);
 const draft=await importedDraft(upload,"installed",declaration,async()=>image);
 expect(draft.bindings.pictures).toHaveLength(2);expect(draft.bindings.pictures?.[0]).toHaveProperty("frame.topology","wrap");expect(draft.images).toEqual({});
 expect(draft.unresolved).toEqual([{name:"tokens",required:false,reasons:["No export"]}]);expect(draft.source).toEqual({id:"a".repeat(64),step:0});
 draft.params.amount=0.1;expect(upload).toEqual(original);
});
test("task image inputs restore owned metadata and coordinates without depending on a paged image list",async()=>{
 const upload=review();upload.import!.inputs.pop();const current=structuredClone(declaration);current.document.actions[0]={...action,kind:"task",images:[{slot:"pictures",role:"source",required:true,many:true}],};delete current.document.actions[0]!.ports;
 const draft=await importedDraft(upload,"installed",current,async id=>{expect(id).toBe(image.id);return image;});
 expect(draft.images.pictures?.map(i=>i.id)).toEqual([image.id,image.id]);expect(draft.images.pictures?.[0]?.coordinates?.topology).toBe("wrap");expect(draft.bindings).toEqual({});
});
test("foreign, in-flight and contradictory import progress cannot open a draft",async()=>{
 for(const edit of [(v:BundleUpload)=>{v.import!.phase="importing";},(v:BundleUpload)=>{v.import!.bundle="f".repeat(64);},(v:BundleUpload)=>{v.import!.step=3;},(v:BundleUpload)=>{v.import!.tool="other";},(v:BundleUpload)=>{v.import!.inputs[0]!.index=9;}]){
  const upload=review();edit(upload);await expect(importedDraft(upload,"installed",declaration,async()=>image)).rejects.toThrow();
 }
});
test("task shared locations remain by value, and native references cannot replace them",async()=>{
 const upload=review();const location={image:{file_sha256:"e".repeat(64),width:10,height:10,projection:"flat",topology:"bounded"},kind:"line",parts:[{column:0.5,width_px:1}],straddles_wrap:false};
 upload.import!.inputs=[{name:"region",index:0,status:"completed",binding:{kind:"location",value:location},reason:null}];
 const current=structuredClone(declaration);current.document.actions[0]={...action,kind:"task",images:[],locations:[{slot:"region",required:true}]};delete current.document.actions[0]!.ports;
 const draft=await importedDraft(upload,"installed",current,async()=>image);expect(draft.bindings.region).toEqual([{kind:"location",value:location}]);
 upload.import!.inputs[0]!.binding={kind:"artifact",reference:{type:"location",tool:"sample"}};await expect(importedDraft(upload,"installed",current,async()=>image)).rejects.toThrow("conflicts");
});
test("a paid imported draft requires a newly chosen cap instead of the archive author's ceiling",async()=>{
 const upload=review();upload.import!.params.max_spend_usd=100;const current=structuredClone(declaration);current.document.actions[0]!.cost.may_spend=true;
 const draft=await importedDraft(upload,"installed",current,async()=>image);expect(draft.params).not.toHaveProperty("max_spend_usd");expect(upload.import!.params.max_spend_usd).toBe(100);
});
test("guided replay preserves fresh origins in operations and core image slots while ordinary imports stay independent",async()=>{
 const upload=review();upload.import!.inputs.pop();const origin={attempt:"fresh-producer",port:"picture",occurrence:2};upload.import!.replay={id:"e".repeat(32),generation:3};upload.import!.inputs[0]!.source=origin;upload.import!.inputs[0]!.changed=true;
 const draft=await importedDraft(upload,"installed",declaration,async()=>image);expect(draft.source.replay).toEqual(upload.import!.replay);expect(draft.bindings.pictures?.[0]?.origin).toEqual(origin);
 const current=structuredClone(declaration);current.document.actions[0]={...action,kind:"task",images:[{slot:"pictures",role:"source",required:true,many:true}]};delete current.document.actions[0]!.ports;
 const task=await importedDraft(upload,"installed",current,async()=>image);expect(task.images.pictures?.[0]?.origin).toEqual(origin);
});
import {encodeWorkspaceInputs} from "../../src/features/workspace/artifact-transfer";
test("core image and operation origins merge without losing either provenance map",()=>{
 const origin={attempt:"fresh-producer",port:"picture",occurrence:2};const other={attempt:"marker-producer",port:"marker",occurrence:0};
 const value=encodeWorkspaceInputs("0.6.0",[{slot:"source",role:"source",required:true}],{source:[{...image,origin}]},{region:[{kind:"location",value:{kind:"line"},origin:other}]});
 expect(value.lineage).toEqual({source:[origin],region:[other]});expect(value.bindings.region?.[0]).not.toHaveProperty("origin");expect(value.images.source).toEqual([image.id]);
});
