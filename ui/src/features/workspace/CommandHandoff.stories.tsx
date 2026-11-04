import {useState,type ComponentProps} from "react";
import {Button} from "../../components/Button";
import type {Meta,StoryObj} from "@storybook/react-vite";
import {LocalApi} from "../../api/client";
import {CommandHandoff} from "./CommandHandoff";
import type {Handoff} from "../../api/handoff";
import {Drawer} from "../../components/Drawer";
const value:Handoff={state:"local",blockers:[],invocation:{action:"shape",arguments:{sigma_px:6,max_spend_usd:0.3},effective:{model:"saved-model",config_identity:`sha256:${"a".repeat(64)}`},requirements:["PROVIDER_KEY","settings.toml"]},arguments:{sigma_px:6,max_spend_usd:0.3},arguments_filename:"wafflery-arguments.json",argv:["/usr/local/bin/study","call","shape","--arguments","/tmp/project/wafflery-arguments.json","--allow-dir","/tmp/project"],command:"cd '/tmp/project' &&\n'/usr/local/bin/study' 'call' 'shape' '--arguments' '/tmp/project/wafflery-arguments.json' '--allow-dir' '/tmp/project'",cwd:"/tmp/project",allowed_dirs:["/tmp/project"],requirements:["PROVIDER_KEY","settings.toml"]};
class PreviewApi extends LocalApi{constructor(private response:Handoff|null){super("http://localhost","unused-story-value");}set(value:Handoff){this.response=value;}override async request(path:string){return this.response?Response.json(path.endsWith("/arguments")?this.response.arguments:this.response):new Promise<Response>(()=>{});}}
const meta={title:"Workspace/CommandHandoff",component:CommandHandoff,args:{id:"saved",tool:{name:"study",version:"1.0.0"},maySpend:true,api:new PreviewApi(value)},render:args=><Drawer open title="Use in your pipeline" onClose={()=>{}}><CommandHandoff {...args}/></Drawer>} satisfies Meta<typeof CommandHandoff>;
export default meta;type Story=StoryObj<typeof meta>;
export const Local:Story={};
export const Loading:Story={args:{api:new PreviewApi(null)}};
export const UnresolvedStore:Story={args:{api:new PreviewApi({...value,state:"unavailable",blockers:["Wafflery started the tool with its own storage settings, and the command can't include those yet."],argv:null,command:null})}};
export const UnavailableInvocation:Story={args:{api:new PreviewApi({...value,state:"unavailable",blockers:["The tool refused this run's inputs, so there is nothing to repeat."],invocation:null,arguments:null,argv:null,command:null,cwd:null,requirements:[],allowed_dirs:[]})}};

function CompletingPreview(args:ComponentProps<typeof CommandHandoff>){
 const [api]=useState(()=>new PreviewApi({...value,state:"unavailable",blockers:["This run hasn't finished, so its final settings aren't known yet."],invocation:null,arguments:null,command:null,argv:null}));
 const [revision,setRevision]=useState("running");
 return <Drawer open title="Use in your pipeline" onClose={()=>{}}><Button onClick={()=>{api.set(value);setRevision("completed");}}>Finish saved run</Button><CommandHandoff {...args} api={api} revision={revision}/></Drawer>;
}
export const Completing:Story={render:args=><CompletingPreview {...args}/>};
class ErrorApi extends LocalApi{override async request():Promise<Response>{throw Error("Owned image bytes are unavailable.");}}
export const Failed:Story={args:{api:new ErrorApi("http://localhost","unused-story-value")}};
