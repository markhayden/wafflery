import { useState, type ComponentProps } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { RunOutcome } from "./RunOutcome";
const record = (code:string | null, usd = 0, unresolved = 0, status = "error") => ({
  execution:status,result:{status,error:code ? {code,reason:"The tool reported this outcome.",retryable:false} : null,
    spend:{usd,unresolved_usd:unresolved,basis:"measured"}},observations:{},raw_response:null,
});
function Example(props:ComponentProps<typeof RunOutcome>) {
  const [opened,setOpened] = useState(false);
  return <div style={{maxWidth:560,padding:"1rem"}}><RunOutcome {...props} onSetup={() => setOpened(true)} />
    {opened && <p>Setup requested</p>}</div>;
}
const meta = {title:"Workspace/RunOutcome",component:RunOutcome,render:args => <Example {...args} />,
  args:{paid:true,attempt:record("credential"),onSetup:()=>{}},
} satisfies Meta<typeof RunOutcome>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Credential:Story = {};
export const Cap:Story = {args:{attempt:record("cap")}};
export const Budget:Story = {args:{attempt:record("budget")}};
export const Stale:Story = {args:{attempt:record("stale_estimate")}};
export const Unresolved:Story = {args:{attempt:record("provider",0.12,0.12)}};
export const Cancelled:Story = {args:{attempt:record(null,0.20,0.12,"cancelled")}};
export const Completed:Story = {args:{attempt:record(null,0.08,0,"completed")}};
export const Interrupted:Story = {args:{attempt:{execution:"unconfirmed",result:null,raw_response:null,observations:{}}}};
export const NoRun:Story = {args:{attempt:{execution:"rejected",result:null,raw_response:null,
  observations:{spent_usd:0,refusal:{reason:"not_ready",run:null,requirements:[{kind:"credential",name:"EXAMPLE_KEY",state:"missing",setup:"Configure credentials outside the playground."}]}}}}};
