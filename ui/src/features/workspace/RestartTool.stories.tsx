import type { Meta, StoryObj } from "@storybook/react-vite";
import { RestartTool } from "./RestartTool";
const meta = {
  title: "Workspace/RestartTool", component: RestartTool,
  args: { busy: false, onRestart: async () => "bb8f5520-a822-4cdc-a9eb-f1a72018e001" },
} satisfies Meta<typeof RestartTool>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Idle: Story = {};
export const Busy: Story = { args: { busy: true } };
export const Disconnected: Story = { args: { onRestart: async () => { throw new Error("The tool could not initialize. Check its configuration before retrying."); } } };
export const Restarting: Story = { args: { onRestart: () => new Promise(() => {}) } };
