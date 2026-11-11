import type { Meta, StoryObj } from "@storybook/react-vite";
import { ToolSetup } from "./ToolSetup";
import type { ToolSession } from "../../api/tool-session";
const session: ToolSession = {
  contract: "0.6.0", tool: { name: "sample-lab", version: "0.1.0" },
  id: "bb8f5520-a822-4cdc-a9eb-f1a72018e000", started: "2026-11-01T10:00:00Z",
  budget_usd: 2, spent_usd: 0.16, unresolved_usd: 0.04, reserved_usd: 0.04, remaining_usd: 1.8,
  allowed_dirs: ["/work/reference images/approved"],
};
const meta = {
  title: "Workspace/ToolSetup", component: ToolSetup,
  args: { tool: session.tool,
    directoryGrants: {requested_dirs:["/work/reference images"], evidence_dirs:["/work/tool/results"]},
    action: { kind: "task", name: "check", title: "Check an image", purpose: "A sample paid action", params_schema: {}, cost: { may_spend: true }, readiness: { state: "ready", requirements: [{ kind: "credential", name: "EXAMPLE_API_KEY", state: "present" }] } },
    loadSession: async () => session },
} satisfies Meta<typeof ToolSetup>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Capped: Story = {};
export const Uncapped: Story = { args: { loadSession: async () => ({ ...session, budget_usd: null, remaining_usd: null }) } };
export const Exhausted: Story = { args: { loadSession: async () => ({ ...session, spent_usd: 2, unresolved_usd: 0, reserved_usd: 0, remaining_usd: 0 }) } };
export const Loading: Story = { args: { loadSession: () => new Promise(() => {}) } };
export const Unavailable: Story = { args: { loadSession: async () => { throw new Error("The tool is disconnected."); } } };

export const Requirements: Story = { args: { action: { ...meta.args.action, readiness: {
  state: "needs_setup", requirements: [
    { kind: "config", name: "settings.toml", state: "missing", setup: "Choose a configuration file outside the playground." },
    { kind: "credential", name: "EXAMPLE_API_KEY", state: "invalid", setup: "Replace the credential outside the playground, then restart the tool." },
    { kind: "env", name: "MODEL_DIR", state: "present" },
    { kind: "model", name: "example-model", state: "valid" },
    { kind: "file", name: "reference collection", state: "unknown" },
    { kind: "dependency", name: "optional backend", state: "missing" },
  ],
} } } };
export const UnavailableAction: Story = { args: { action: { ...meta.args.action, readiness: {
  state: "unavailable", requirements: [{ kind: "dependency", name: "optional backend", state: "missing", setup: "This build does not include the backend." }],
} } } };

export const NarrowedAccess: Story = {};
export const NoFileAccess: Story = {args:{loadSession:async () => ({...session,allowed_dirs:[]})}};
/** A checking key the action can run without: the action stays ready, and the key says what it
 * is for. */
export const OptionalRequirement: Story = { args: { action: { ...meta.args.action, answers: "verdict", readiness: {
  state: "ready", requirements: [
    { kind: "credential", name: "EXAMPLE_CHECK_KEY", state: "missing", optional: true, for: "checking the result",
      setup: "Set EXAMPLE_CHECK_KEY where the tool starts, then refresh. Without it the result is not checked." },
  ],
} } } };
/** Like rascal's remove_prompt: settings every call needs, one model requirement per choice and an
 * optional checking key. Only the chosen model's requirement is listed; picking one that is not
 * installed lists it as needed. */
export const ChoiceRequirements: Story = { args: { params: { model: "birefnet" }, action: { ...meta.args.action,
  params_schema: { type: "object", properties: { model: { type: "string", enum: ["tiny", "birefnet"], default: "tiny" } } },
  readiness: { state: "ready", requirements: [
    { kind: "config", name: "rascal.toml", state: "present" },
    { kind: "env", name: "RASCAL_TOOLS_MODELS", state: "present" },
    { kind: "model", name: "segmenter:tiny", state: "present", for_choice: { param: "model", value: "tiny" } },
    { kind: "model", name: "segmenter:birefnet", state: "missing", for_choice: { param: "model", value: "birefnet" },
      setup: "BiRefNet is not installed. Get it in the tool's setup." },
    { kind: "credential", name: "ANTHROPIC_API_KEY", state: "missing", optional: true, for: "checking the result" },
  ] } } } };
