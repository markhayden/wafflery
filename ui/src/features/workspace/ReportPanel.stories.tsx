import { useState, type ComponentProps } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { Drawer } from "../../components/Drawer";
import { Button } from "../../components/Button";
import { ReportPanel } from "./ReportPanel";
import type { ReportSnapshot } from "./report-owner";
const initial: ReportSnapshot = {view:null,url:null,pending:false,uncertain:false,error:null};
const view = {id:"a".repeat(32),attempt:"saved",name:"overview",kind:"server" as const,
  phase:"ready" as const,exit_code:null,error:null,idle_expired:false,diagnostics:"Viewer listening on loopback.",diagnostics_truncated:false};
const preview = `data:text/html,${encodeURIComponent('<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width"><style>body{font:15px/1.6 system-ui;background:#171918;color:#e1e4e1;padding:24px}h1{font-size:22px;font-weight:500}p{color:#a0a6a1}svg{width:100%;margin-top:16px}</style><h1>Image review</h1><p>One finding · Saved report</p><svg viewBox="0 0 500 220" role="img" aria-label="Illustrative image finding"><rect width="500" height="220" fill="#303833"/><path d="M250 12v196" stroke="#ddd" stroke-width="3" stroke-dasharray="6 6"/><rect x="218" y="42" width="64" height="136" fill="none" stroke="#e2b674" stroke-width="2"/></svg>')}`;
function Example(props: ComponentProps<typeof ReportPanel>) {
  const [open,setOpen] = useState(true), [selection,setSelection] = useState(props.selection);
  const [snapshot,setSnapshot] = useState(props.snapshot), [forced,setForced] = useState(0);
  return <>
    <Button onClick={() => setOpen(true)}>Inspect run</Button>
    <Drawer open={open} onClose={() => setOpen(false)} title="Run details" description="sample-lab / mark_band · saved">
      <ReportPanel {...props} selection={selection} snapshot={snapshot} onSelect={setSelection}
        onOpen={() => setSnapshot({...initial,view,url:preview})}
        onClose={() => setSnapshot({...initial,view:{...view,phase:"exited"}})}
        onForce={() => { setForced(n=>n+1); setSnapshot({...initial,view:{...view,phase:"exited"}}); }} />
      <p>Force stops confirmed: {forced}</p>
    </Drawer>
  </>;
}
const meta = {title:"Workspace/ReportPanel",component:ReportPanel,render:args=><Example {...args}/>,
  args:{attempt:"saved",choices:[{name:"overview",kind:"server"},{name:"document",kind:"document"}],selection:"overview",snapshot:initial,
    onSelect:()=>{},onOpen:()=>{},onClose:()=>{},onRecover:()=>{},onForce:()=>{}},
} satisfies Meta<typeof ReportPanel>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Empty: Story = {};
export const Ready: Story = {args:{snapshot:{...initial,view,url:preview}}};
export const Starting: Story = {args:{snapshot:{...initial,view:{...view,phase:"starting"}}}};
export const Failed: Story = {args:{snapshot:{...initial,view:{...view,phase:"exited",error:"The report could not start.",exit_code:17,diagnostics:"The tool reported a missing viewer dependency."}}}};
export const NeedsForce: Story = {args:{snapshot:{...initial,view:{...view,phase:"needs_force",error:"The viewer did not stop after its close request."}}}};
export const Unconfirmed: Story = {args:{snapshot:{...initial,uncertain:true,error:"The open reply was lost. Quit and reopen Wafflery before opening another report."}}};
export const Expired: Story = {args:{snapshot:{...initial,view:{...view,phase:"exited",idle_expired:true}}}};
export const Unavailable: Story = {args:{choices:[],selection:""}};
