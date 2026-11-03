import {expect,test} from "bun:test";
import {activityChoices,activityKinds,activitySummary,firstActivity} from "../../src/features/workspace/activities";
import type {Action} from "../../src/features/actions/model";
const action=(name:string,kind:Action["kind"],group?:string):Action=>({name,title:name,purpose:"A declared activity",
  kind,...(group?{group}:{}),params_schema:{},cost:{may_spend:false},readiness:{state:"ready",requirements:[]}});
test("kinds and groups derive from metadata, not action names or ordering",()=>{
  const actions=[action("seam","utility"),action("b","operation","other"),action("a","operation","renamed"),
    action("c","operation","other"),action("line","task"),action("ungrouped","operation")];
  expect(activityKinds(actions)).toEqual(["task","operation","utility"]);
  expect(firstActivity(actions)?.name).toBe("line");
  expect(activityChoices(actions,"operation",["renamed","empty","other"]).map(c=>[c.id,c.group]))
    .toEqual([["ungrouped",undefined],["a","renamed"],["b","other"],["c","other"]]);
  expect(activityChoices(actions,"task",[]).map(c=>c.id)).toEqual(["line"]);
});
test("empty kinds stay empty and unavailable actions remain selectable for setup",()=>{
  const item=action("inspect","operation","find");item.readiness.state="unavailable";
  expect(activityKinds([])).toEqual([]);expect(firstActivity([])).toBeUndefined();
  expect(firstActivity([item])?.name).toBe("inspect");
  expect(activityChoices([item],"task",[])).toEqual([]);
  expect(activityChoices([item],"operation",["find"])[0]?.disabled).not.toBe(true);
});

test("operation trigger summaries use declared port types rather than long purpose prose",()=>{
 const operation={...action("Refine","operation"),purpose:"Detailed purpose that belongs in the chooser and input form.",ports:{in:[{name:"source",type:"image",required:true},{name:"markers",type:"location",many:true,required:false}],out:[{name:"mask",type:"opaque:Mask",required:true}]}};
 expect(activitySummary(operation)).toBe("Image + Location[] → opaque:Mask");
 expect(activitySummary({...operation,ports:{in:[],out:[]}})).toBe("Parameters → Result");
});
test("task summaries stay compact in tools that also declare operations",()=>{
 expect(activitySummary({...action("Repair","task"),purpose:"Long task purpose.",images:[{slot:"source",role:"source",required:true}]})).toBe("Image → Result");
});
