import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import type { Attempt } from "../../api/workspace";
import { LocalApi } from "../../api/client";
import { Drawer } from "../../components/Drawer";
import { Button } from "../../components/Button";
import { Inspector } from "./Inspector";
import { Pipeline } from "./Pipeline";
import { ReportOwner } from "./report-owner";

const attempt: Attempt = {
  id:"saved-example",sequence:1,created_ms:0,
  request:{id:"saved-example",tool:"sample-lab",action:"mark_band",revision:"example",params:{label:"A saved experiment",strength:0.5},images:{}},
  arguments:{label:"A saved experiment",strength:0.5},
  declaration:{revision:"example",generation:1,schemas:{},document:{contract:"0.4.0",
    tool:{name:"sample-lab",version:"0.1.0",purpose:"Illustrative offline tool."},
    actions:[{name:"mark_band",title:"Add a test band",purpose:"Illustrative image band.",params_schema:{},
      cost:{may_spend:false},readiness:{state:"ready",requirements:[]}}]}},
  inputs:{},outputs:[],wire:{method:"tools/call",params:{name:"mark_band",arguments:{label:"A saved experiment",strength:0.5}}},
  raw_response:{status:"ok",spend:{usd:0}},result:{status:"ok"},execution:"completed",
  preservation:{contract:"complete",native:"not_saved",report:"not_saved"},journal:[{event:"completed"}],notices:[],
};
const unavailable = async (): Promise<never> => { throw new Error("No report process is launched by this story."); };
class PreviewApi extends LocalApi {
  override async request(): Promise<Response> { throw new Error("Archive download requires a running local workspace."); }
}
function EvidenceStory({pipeline = false}: {pipeline?: boolean}) {
  const [open,setOpen] = useState(true);
  const [reports] = useState(() => new ReportOwner({current:async()=>null,open:unavailable,read:unavailable,
    close:unavailable,force:unavailable,activity:unavailable,frame:unavailable}));
  const [api] = useState(() => new PreviewApi("http://localhost", "unused-story-value"));
  return <><Button onClick={() => setOpen(true)}>Inspect saved run</Button>
    <Drawer open={open} onClose={() => setOpen(false)} title={pipeline ? "Use in your pipeline" : "Run details"}
      description="sample-lab / mark_band · saved-example">
      {pipeline ? <Pipeline attempt={attempt} api={api} /> : <Inspector attempt={attempt} reports={reports} />}
    </Drawer></>;
}
const meta = {title:"Workspace/Evidence",component:EvidenceStory,parameters:{layout:"fullscreen"}} satisfies Meta<typeof EvidenceStory>;
export default meta;
type Story = StoryObj<typeof meta>;
export const RunDetails: Story = {};
export const UseInPipeline: Story = {args:{pipeline:true}};
