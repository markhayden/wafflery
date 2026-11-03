import type {Meta,StoryObj} from "@storybook/react-vite";
import {LocalApi} from "../../api/client";
import type {BundleRecord} from "../../api/bundles";
import {Drawer} from "../../components/Drawer";
import {ChainDependencies} from "./BundleChain";
import {BundleReview,InvocationBundles} from "./InvocationBundles";
const hash="a".repeat(64);
const saved:BundleRecord={id:hash,sha256:hash,size:24576,workspace_dependent:false,members:[{name:`inputs/${"b".repeat(64)}.png`,sha256:"b".repeat(64),bytes:1024},{name:`artifacts/sha256-${"c".repeat(64)}.json`,sha256:"d".repeat(64),bytes:512}],manifest:{bundle_version:1,steps:[{contract:"0.4.0",tool:{name:"study",version:"1.0.0"},invocation:{action:"rasterize",arguments:{width_px:16},effective:{model:null,config_identity:`sha256:${"1".repeat(64)}`},requirements:["settings.toml"]},result:{},inputs:[],artifacts:[{port:"marker",artifact:{type:"location",identity:`sha256:${"c".repeat(64)}`},member:`artifacts/sha256-${"c".repeat(64)}.json`,sha256:"d".repeat(64)}],reference_only:[],unportable:[]}]}};
class PreviewApi extends LocalApi{
 constructor(private mode:"ready"|"partial"|"loading"|"failed"|"empty"){super("http://localhost","unused-story-value");}
 override async request(path:string):Promise<Response>{
  if(this.mode==="loading")return new Promise(()=>{});if(this.mode==="failed")throw Error("Saved invocation bundle fingerprint failed verification.");
  const record=structuredClone(saved);if(this.mode==="partial"){record.workspace_dependent=true;record.manifest.steps[0]!.unportable=[{port:"mask",artifact:{type:"study:Mask"},reason:"No owned native export is available. Preserve it before exporting a new bundle."}];}
  if(path==="/api/bundles")return Response.json({items:this.mode==="empty"?[]:[{id:hash,sequence:1,size:record.size,workspace_dependent:record.workspace_dependent,tool:{name:"study",version:"1.0.0"},action:"rasterize"}],cursor:null});
  return Response.json(record);
 }
}
const meta={title:"Workspace/InvocationBundles",component:InvocationBundles,args:{api:new PreviewApi("ready")},render:args=><Drawer open title="Saved invocation bundles" onClose={()=>{}}><InvocationBundles {...args}/></Drawer>} satisfies Meta<typeof InvocationBundles>;
export default meta;type Story=StoryObj<typeof meta>;
export const Ready:Story={};
export const Partial:Story={args:{api:new PreviewApi("partial")}};
export const Loading:Story={args:{api:new PreviewApi("loading")}};
export const Failed:Story={args:{api:new PreviewApi("failed")}};
export const Empty:Story={args:{api:new PreviewApi("empty")}};

const linked=structuredClone(saved);linked.manifest.steps.push(structuredClone(saved.manifest.steps[0]!));linked.chain={problem:null,edges:[{consumer:1,name:"mask",index:0,producer:{step:0,port:"mask",item:0,aliases:[{profile:"artifacts",port:"mask",index:0,item:0}]}}]};
export const Chain:Story={render:()=> <Drawer open title="Saved invocation bundles" onClose={()=>{}}><BundleReview bundle={linked} onDownload={()=>{}}/></Drawer>};
export const MixedTools:Story={render:()=>{const mixed=structuredClone(linked);mixed.manifest.steps[1]!.tool.name="reviewer";return <Drawer open title="Saved invocation bundles" onClose={()=>{}}><BundleReview bundle={mixed} onDownload={()=>{}}/></Drawer>;}};
export const AmbiguousChain:Story={render:()=> <Drawer open title="Saved invocation bundles" onClose={()=>{}}><ChainDependencies bundle={{...linked,chain:{edges:[],problem:"Input mask has more than one matching earlier output."}}}/></Drawer>};
class SelectionApi extends LocalApi{
 constructor(){super("http://localhost","unused-story-value");}
 override async request(path:string):Promise<Response>{
  if(path.startsWith("/api/attempts?"))return Response.json({items:[{id:"consumer-run",sequence:2,created_ms:0,tool:"study",action:"feather",execution:"completed"},{id:"producer-run",sequence:1,created_ms:0,tool:"study",action:"rasterize",execution:"completed"}],cursor:null});
  return Response.json({items:[],cursor:null});
 }
}
export const Selection:Story={args:{api:new SelectionApi()}};
