import type {Meta,StoryObj} from "@storybook/react-vite";
import {LocalApi} from "../../api/client";
import type {BundleRecord} from "../../api/bundles";
import type {Attempt} from "../../api/workspace";
import {Drawer} from "../../components/Drawer";
import {BundleSource} from "./BundleSource";
const source={id:"a".repeat(64),step:0};
const record:BundleRecord={id:source.id,sha256:source.id,size:2048,workspace_dependent:false,members:[],manifest:{bundle_version:1,steps:[{contract:"0.5.0",tool:{name:"study",version:"1.0.0"},invocation:{action:"soften",arguments:{amount:0.25},effective:{},requirements:[]},result:{artifacts:{out:[{port:"mask",artifact:{type:"study:Mask",identity:`sha256:${"b".repeat(64)}`}}]}},inputs:[],artifacts:[],reference_only:[],unportable:[]}]}};
class Preview extends LocalApi {constructor(){super("http://localhost","unused-story-value");}override async request(){return Response.json(record);}}
const attempt:Attempt={id:"new-experiment",sequence:2,created_ms:0,request:{id:"new-experiment",tool:"study",action:"soften",revision:"r",params:{amount:0.5},images:{},source_bundle:source},arguments:{},declaration:{document:{contract:"0.5.0",tool:{name:"study",version:"1.0.0",purpose:"Test"},actions:[]},revision:"r",generation:0,schemas:{}},inputs:{},wire:null,raw_response:null,result:{artifacts:{out:[{port:"mask",artifact:{type:"study:Mask",identity:`sha256:${"c".repeat(64)}`}}]}},execution:"completed",preservation:{contract:"complete",native:"preserved",report:"unsupported"},outputs:[],journal:null,notices:[]};
const meta={title:"Workspace/BundleSource",component:BundleSource,args:{api:new Preview(),source,unresolved:[],canResolve:():boolean=>true,resolve:()=>{}},render:args=><Drawer open title="Imported step" onClose={()=>{}}><BundleSource {...args}/></Drawer>} satisfies Meta<typeof BundleSource>;
export default meta;type Story=StoryObj<typeof meta>;
export const Ready:Story={};
export const Unresolved:Story={args:{unresolved:[{name:"mask",required:true,reasons:["No portable native file was included. Select a replacement before Run."]},{name:"references",required:false,reasons:["Reference images were omitted deliberately."]}],canResolve:()=>false}};
export const ChangedIdentity:Story={args:{attempt}};
