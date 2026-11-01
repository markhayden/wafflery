import type { Meta, StoryObj } from "@storybook/react";
import { Examples } from "./Examples";
import type { Example } from "../../api/examples";
const example: Example = {
  id: "example",
  sequence: 1,
  name: "A saved experiment with a longer name",
  notes: "Review the same inputs later.",
  request: {
    id: "source",
    tool: "sample-lab",
    action: "mark_band",
    revision: "fixture",
    params: { label: "Saved label" },
    images: {},
  },
  inputs: {},
};
const meta = {
  title: "Workspace/Examples",
  component: Examples,
  args: {
    selectedId: "source",
    onOpen: () => {},
    client: {
      list: async () => ({
        items: [
          {
            id: example.id,
            sequence: 1,
            name: example.name,
            tool: "sample-lab",
            action: "mark_band",
          },
        ],
        cursor: null,
      }),
      open: async () => example,
      save: async () => example,
    },
  },
} satisfies Meta<typeof Examples>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Saved: Story = {};
export const Empty: Story = {
  args: {
    selectedId: undefined,
    client: {
      ...meta.args.client,
      list: async () => ({ items: [], cursor: null }),
    },
  },
};
export const Unavailable: Story = {
  args: {
    client: {
      ...meta.args.client,
      list: async () => {
        throw new Error("The library is unavailable.");
      },
    },
  },
};
