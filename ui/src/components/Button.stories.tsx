import type { Meta, StoryObj } from "@storybook/react-vite";
import { Button } from "./Button";

const meta = {
  title: "Controls/Button",
  component: Button,
  args: { children: "Run" },
} satisfies Meta<typeof Button>;
export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
export const Disabled: Story = { args: { disabled: true } };
export const Busy: Story = { args: { busy: true, children: "Running…" } };
export const LongLabel: Story = {
  args: { children: "Download this run’s configuration and all saved inputs" },
};
export const Primary: Story = { args: { variant: "primary", children: "Run" } };
export const Destructive: Story = { args: { variant: "destructive", children: "Remove" } };
