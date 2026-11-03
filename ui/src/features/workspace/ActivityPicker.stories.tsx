import type {Meta,StoryObj} from "@storybook/react-vite";
import {useState} from "react";
import type {Action,ActivityKind} from "../actions/model";
import {ActivityPicker} from "./ActivityPicker";
const action=(name:string,kind:ActivityKind,group?:string):Action=>({name,title:name,kind,
  ...(group?{group}:{}),purpose:"Inspect this independent part using the tool’s declared inputs.",
  params_schema:{},cost:{may_spend:false},readiness:{state:"ready",requirements:[]}});
const actions=[action("Make a study","task"),action("Measure a boundary","operation","analyze"),
  action("Build a shape","operation","create"),action("Refine a shape","operation","create"),
  action("Inspect metadata","utility")];
const meta={title:"Workspace/ActivityPicker",component:ActivityPicker,
  args:{actions,groups:["create","analyze"],kind:"operation",value:"Measure a boundary",onKind:()=>{},onAction:()=>{}},
  render:args=>{
    const [kind,setKind]=useState(args.kind),[value,setValue]=useState(args.value);
    return <ActivityPicker {...args} kind={kind} value={value} onAction={setValue} onKind={kind=>{
      setKind(kind);setValue(args.actions.find(action=>action.kind===kind)?.name??"");
    }}/>;
  },
} satisfies Meta<typeof ActivityPicker>;
export default meta;
type Story=StoryObj<typeof meta>;
export const Grouped:Story={};
export const TaskOnly:Story={args:{actions:[actions[0]!],kind:"task",value:"Make a study"}};
export const EmptyKind:Story={args:{kind:"utility",value:"",actions:actions.filter(action=>action.kind!=="utility")}};
export const LongLabels:Story={args:{actions:[action("Inspect an unusually long declared operation title that must wrap safely even on narrow screens","operation","analyze")],kind:"operation",value:""}};
export const NeedsSetup:Story={args:{actions:[{...actions[1]!,readiness:{state:"unavailable",requirements:[
  {kind:"dependency",name:"marker-engine",state:"missing",setup:"Install the marker engine to use this operation."}]}}]}};
export const LongPurpose:Story={args:{actions:[actions[0]!,{...action("Refine mask","operation","refine"),purpose:"Refine the selected mask independently. ".repeat(15),ports:{in:[{name:"mask",type:"rascal:Mask",required:true}],out:[{name:"mask",type:"rascal:Mask",required:true}]}}],value:"Refine mask"}};

export const LongTaskPurpose:Story={args:{actions:[{...action("Repair a sample","task"),purpose:"A complete sample task with detailed purpose. ".repeat(15),images:[{slot:"source",role:"source",required:true}]},actions[1]!],kind:"task",value:"Repair a sample"}};
