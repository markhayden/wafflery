import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { WorkspaceFrame } from "./WorkspaceFrame";
import { Picker } from "./Picker";
const choices = [{id:"sample",title:"sample-lab",description:"Version 0.1.0"},
  {id:"long",title:"A tool with a deliberately long name",description:"Version 0.1.0"}];
function Example() {
  const [tool,setTool] = useState("sample");
  return <WorkspaceFrame connection="Connected locally" connected
    toolPicker={<Picker header label="Tool" choices={choices} value={tool} onChange={setTool} />}>
    <h1>{choices.find(choice => choice.id === tool)?.title}</h1>
    <p className="workspace-intro">Try a tool. Inspect its output. Keep the evidence.</p>
  </WorkspaceFrame>;
}
const meta = {title:"Components/WorkspaceFrame",component:WorkspaceFrame,
  args:{connection:"Connected locally",children:null},
  parameters:{layout:"fullscreen"},render:()=><Example />,
} satisfies Meta<typeof WorkspaceFrame>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Connected: Story = {};
export const Connecting: Story = {render:()=> <WorkspaceFrame connection="Connecting…"><h1>Your workshop</h1></WorkspaceFrame>};
export const Unavailable: Story = {render:()=> <WorkspaceFrame connection="The local workspace is unavailable."><h1>Your workshop</h1><p>Restart Wafflery in your terminal to reconnect.</p></WorkspaceFrame>};
