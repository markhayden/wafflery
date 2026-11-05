import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { ChoiceCard, ChoiceCards } from "./ChoiceCard";

const items = [
  ["Recorded call", "The exact request Wafflery sent to the tool."],
  ["Resolved invocation", "The arguments and settings the tool ran with, defaults filled in."],
  ["Journal", "The tool's own step-by-step record of the run."],
  ["Session", "The tool session's spending before this run, and what the run reported."],
  ["Reports", "Report pages the tool wrote for this run."],
] as const;
function Example({ min, layout }: { min?: string; layout?: "grid" | "list" }) {
  const [selected, setSelected] = useState(0);
  return <div style={{ maxWidth: "40rem" }}><ChoiceCards label="Evidence records" {...(min ? { min } : {})} {...(layout ? { layout } : {})}>
    {items.map(([title, description], index) => <ChoiceCard key={title} title={title} description={description} selected={selected === index} onClick={() => setSelected(index)} />)}
  </ChoiceCards></div>;
}
const meta = { title: "Controls/ChoiceCard", component: Example } satisfies Meta<typeof Example>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Grid: Story = {};
export const OneColumn: Story = { args: { min: "40rem" } };
export const List: Story = { args: { layout: "list" } };
export const Single: Story = { render: () => <div style={{ maxWidth: "20rem" }}><ChoiceCard title="Journal" description="The tool's own step-by-step record of the run." /></div> };
