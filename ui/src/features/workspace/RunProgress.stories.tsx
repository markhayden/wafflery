import type { Meta, StoryObj } from "@storybook/react-vite";
import { RunProgress } from "./RunProgress";
import { RunStatus } from "./RunStatus";

const meta = { title: "Workspace/RunProgress", component: RunProgress, args: { action: "Line", message: "step rasterize: ok; 3 s", cancelling: false, onCancel: () => {} } } satisfies Meta<typeof RunProgress>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Running: Story = {};
export const Starting: Story = { args: { message: undefined } };
export const Cancelling: Story = { render: (args) => <RunProgress action={args.action} message={args.message} cancelling /> };
/** The Result panel heading in each state: the status and, beside it, the reported spend. */
export const Headings: Story = { render: () => <div style={{ display: "grid", gap: "1rem", maxWidth: "36rem" }}>
  {[["running", undefined], ["completed", { status: "completed", spend: { usd: 0, unresolved_usd: 0, basis: "measured" } }],
    ["completed", { status: "invalid", spend: { usd: 0, unresolved_usd: 0, basis: "measured" } }],
    ["completed", { status: "cancelled", spend: { usd: 0.2, unresolved_usd: 0.12, basis: "measured" } }]].map(([execution, result], index) =>
    <div key={index} style={{ display: "flex", justifyContent: "space-between", alignItems: "center", padding: "1rem 1.25rem", border: "1px solid var(--color-line)", borderRadius: "0.5rem" }}>
      <strong style={{ fontWeight: 500 }}>Result</strong>
      <RunStatus attempt={{ execution: execution as string, result: (result ?? null) as never, observations: null as never, raw_response: null as never }} />
    </div>)}
</div> };
