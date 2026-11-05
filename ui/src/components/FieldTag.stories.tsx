import type { Meta, StoryObj } from "@storybook/react-vite";
import { FieldTag } from "./FieldTag";

function Labels() {
  const row = (label: string, tag: React.ReactNode) => <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", fontWeight: 500 }}>{label}{tag}</div>;
  return <div style={{ display: "grid", gap: "0.75rem" }}>
    {row("Source image", <FieldTag required />)}
    {row("Filler", <FieldTag required={false} />)}
    {row("Images", <FieldTag required={false} extra="List" />)}
  </div>;
}
const meta = { title: "Controls/FieldTag", component: Labels } satisfies Meta<typeof Labels>;
export default meta;
type Story = StoryObj<typeof meta>;
/** Required, optional, and optional with a note, each beside its label. */
export const Beside: Story = {};
