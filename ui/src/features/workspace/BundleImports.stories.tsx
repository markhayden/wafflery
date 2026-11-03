import type {Meta,StoryObj} from "@storybook/react-vite";
import {LocalApi} from "../../api/client";
import type {BundleUpload,ImportProgress} from "../../api/bundle-imports";
import {Drawer} from "../../components/Drawer";
import {BundleImports} from "./BundleImports";
const id="b".repeat(32),hash="a".repeat(64);
const progress:ImportProgress={phase:"failed",bundle:hash,step:0,tool:"study",action:"collect",params:{},error:"The second native file failed the tool's identity check. The first input is retained.",inputs:[{name:"masks",index:0,status:"completed",binding:{kind:"artifact",reference:{type:"study:Mask",identity:`sha256:${"c".repeat(64)}`}},reason:null},{name:"masks",index:1,status:"failed",binding:null,reason:null}]};
const upload:BundleUpload={id,import:null,record:{id:hash,sha256:hash,size:24576,workspace_dependent:true,members:[],manifest:{bundle_version:1,steps:[{contract:"0.4.0",tool:{name:"study",version:"1.0.0"},invocation:{action:"collect",arguments:{masks:[]},effective:{config_identity:`sha256:${"1".repeat(64)}`},requirements:["STUDY_API_KEY"]},result:{},inputs:[],artifacts:[],reference_only:[{port:"source",why:"Choose an image explicitly on replay."}],unportable:[]}]}}};
const image={file_sha256:"e".repeat(64),width:160,height:100,projection:"flat",topology:"bounded"};
const refs=["c","d"].map(value=>({type:"study:Mask",schema_version:1,tool:"study",identity:`sha256:${value.repeat(64)}`,frame:image,resource:`study://artifact/sha256:${value.repeat(64)}`}));
const step=upload.record.manifest.steps[0]!;
step.invocation.arguments={masks:refs,source:{expect:image}};
step.artifacts=refs.map((artifact,index)=>({port:"masks",artifact,member:`artifacts/sha256-${["c","d"][index]!.repeat(64)}.bin`,sha256:["f","1"][index]!.repeat(64)}));
upload.record.members=step.artifacts.map(artifact=>({name:artifact.member,sha256:artifact.sha256,bytes:512}));
class PreviewApi extends LocalApi {
 constructor(private mode:"review"|"partial"|"importing"|"loading"|"failed"){super("http://localhost","unused-story-value");}
 override async request(path:string):Promise<Response>{
  if(this.mode==="loading")return new Promise(()=>{});if(this.mode==="failed")throw Error("Invocation bundle fingerprint failed verification.");
  if(path==="/api/state")return Response.json({tools:[],attempts:[]});
  if(path==="/api/bundle-reviews")return Response.json({items:[{id,bundle:hash,phase:this.mode==="review"?"review":this.mode==="partial"?"failed":"importing"}]});
  const value=structuredClone(upload);if(this.mode==="partial")value.import=progress;
  if(this.mode==="importing")value.import={...progress,phase:"importing",error:null,inputs:progress.inputs.map(i=>({...i,status:i.index===0?"completed":"importing"}))};
  return Response.json(path.endsWith("/progress")?value.import:value);
 }
}
const meta={title:"Workspace/BundleImports",component:BundleImports,args:{api:new PreviewApi("review")},render:args=><Drawer open title="Import invocation bundle" onClose={()=>{}}><BundleImports {...args}/></Drawer>} satisfies Meta<typeof BundleImports>;
export default meta;type Story=StoryObj<typeof meta>;
export const Review:Story={};
export const Partial:Story={args:{api:new PreviewApi("partial")}};
export const Importing:Story={args:{api:new PreviewApi("importing")}};
export const Loading:Story={args:{api:new PreviewApi("loading")}};
export const Failed:Story={args:{api:new PreviewApi("failed")}};
