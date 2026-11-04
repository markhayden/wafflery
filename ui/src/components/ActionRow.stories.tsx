import type { Meta, StoryObj } from "@storybook/react-vite";
import { ActionRow } from "./ActionRow";
import { Button } from "./Button";

const meta = {
  title: "Controls/ActionRow",
  component: ActionRow,
  render: (args) => <div style={{ maxWidth: "28rem" }}>
    <label style={{ display: "grid", gap: "0.25rem" }}>Image coordinates
      <select><option>Flat image · bounded edges</option></select></label>
    <ActionRow {...args}><Button>↑</Button><Button>↓</Button><Button variant="destructive">Remove</Button></ActionRow>
  </div>,
} satisfies Meta<typeof ActionRow>;
export default meta;
type Story = StoryObj<typeof meta>;

export const UnderAField: Story = {};
export const PanelFooter: Story = {
  render: () => <div style={{ display: "flex", alignItems: "center", gap: "1rem", maxWidth: "40rem" }}>
    <p style={{ margin: 0 }}>Saved attempt · 90d303e5</p>
    <ActionRow flush><Button>Inspect run</Button><Button>Use in your pipeline</Button></ActionRow>
  </div>,
};
export const Narrow: Story = { parameters: { viewport: { defaultViewport: "mobile1" } } };
