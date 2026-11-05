import { useState, type ComponentProps } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { CostReview } from "./CostReview";
import type { Estimate } from "../../api/estimates";
const quote: Estimate = {
  effective: {model:"scripted-model", config_identity:`sha256:${"a".repeat(64)}`},
  worst_case_usd:0.12, cap_usd:0.5, allowed:true, refused:null,
  readiness:{state:"ready",requirements:[]}, expires:"2030-01-01T00:00:00Z",
  session:{contract:"0.5.0",tool:{name:"sample-lab",version:"0.1.0"},
    id:"22222222-2222-4222-8222-222222222222",started:"2026-11-01T00:00:00Z",
    budget_usd:2,spent_usd:0.16,reserved_usd:0.04,unresolved_usd:0.04,remaining_usd:1.8,allowed_dirs:[]},
};
function Example(props: ComponentProps<typeof CostReview>) {
  const [cap, setCap] = useState(props.cap), [count, setCount] = useState(0), [setup, setSetup] = useState(false);
  return <form style={{maxWidth:560, padding:"1rem"}} onSubmit={event => event.preventDefault()}>
    <CostReview {...props} cap={cap} onCapChange={setCap} onEstimate={() => setCount(value => value + 1)} onSetup={() => setSetup(true)} />
    <p style={{marginTop:"2rem"}}>Estimates requested: {count}</p>
    {setup && <p>Setup requested</p>}
  </form>;
}
const meta = { title:"Workspace/CostReview", component:CostReview, render:args => <Example {...args} />,
  args:{cap:"0.5",state:{kind:"quoted",estimate:quote},onCapChange:()=>{},onEstimate:()=>{},onSetup:()=>{}},
} satisfies Meta<typeof CostReview>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Ready: Story = {};
export const Interactive: Story = {args:{cap:"",state:{kind:"idle"}}};
export const Pending: Story = {args:{state:{kind:"pending"}}};
export const Stale: Story = {args:{state:{kind:"stale"}}};
export const Failed: Story = {args:{cap:"2",state:{kind:"error",message:"The cap exceeds the configured limit."}}};
export const NeedsSetup: Story = {args:{state:{kind:"quoted",estimate:{...quote,allowed:false,refused:"readiness",worst_case_usd:null,cap_usd:null,
  effective:{model:null,config_identity:null},readiness:{state:"needs_setup",requirements:[{kind:"env",name:"MODEL_DIR",state:"missing",setup:"Set MODEL_DIR before starting the tool."}]}}}}};
export const OverCap: Story = {args:{cap:"0.01",state:{kind:"quoted",estimate:{...quote,allowed:false,refused:"cap",cap_usd:0.01}}}};
export const BudgetRefused: Story = {args:{state:{kind:"quoted",estimate:{...quote,allowed:false,refused:"budget",session:{...quote.session,budget_usd:0.25,remaining_usd:0.05}}}}};
export const Uncapped: Story = {args:{state:{kind:"quoted",estimate:{...quote,session:{...quote.session,budget_usd:null,remaining_usd:null}}}}};
export const TinyCost: Story = {args:{state:{kind:"quoted",estimate:{...quote,worst_case_usd:0.00000001}}}};
export const LedgerChanged: Story = {args:{session:{...quote.session,spent_usd:1.96,remaining_usd:0}}};
export const SessionChanged: Story = {args:{session:{...quote.session,id:"33333333-3333-4333-8333-333333333333"}}};

export const Unavailable: Story = {args:{state:{kind:"quoted",estimate:{...quote,allowed:false,refused:"readiness",worst_case_usd:null,cap_usd:null,
  effective:{model:null,config_identity:null},readiness:{state:"unavailable",requirements:[{kind:"dependency",name:"optional backend",state:"missing",setup:"This build does not include the backend."}]}}}}};
