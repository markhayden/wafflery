import type { Meta, StoryObj } from "@storybook/react-vite";
import { ToolSetup } from "./ToolSetup";
import type { ToolSession } from "../../api/tool-session";
const session: ToolSession = {
  contract: "0.3.0", tool: { name: "sample-lab", version: "0.1.0" },
  id: "bb8f5520-a822-4cdc-a9eb-f1a72018e000", started: "2026-11-01T10:00:00Z",
  budget_usd: 2, spent_usd: 0.16, unresolved_usd: 0.04, reserved_usd: 0.04, remaining_usd: 1.8,
};
const meta = {
  title: "Workspace/ToolSetup", component: ToolSetup,
  args: { tool: session.tool,
    action: { name: "check", title: "Check an image", purpose: "A sample paid action", params_schema: {}, cost: { may_spend: true }, readiness: { state: "ready", requirements: [{ kind: "credential", name: "EXAMPLE_API_KEY", state: "present" }] } },
    loadSession: async () => session },
} satisfies Meta<typeof ToolSetup>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Capped: Story = {};
export const Uncapped: Story = { args: { loadSession: async () => ({ ...session, budget_usd: null, remaining_usd: null }) } };
export const Exhausted: Story = { args: { loadSession: async () => ({ ...session, spent_usd: 2, unresolved_usd: 0, reserved_usd: 0, remaining_usd: 0 }) } };
export const Loading: Story = { args: { loadSession: () => new Promise(() => {}) } };
export const Unavailable: Story = { args: { loadSession: async () => { throw new Error("The tool is disconnected."); } } };
