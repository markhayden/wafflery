import { useState, type ComponentProps } from "react";
import { Drawer } from "../../components/Drawer";
import { Button } from "../../components/Button";
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
function Preview(args: ComponentProps<typeof Examples>) {
  const [open,setOpen] = useState(true);
  return <><Button onClick={() => setOpen(true)}>Open examples</Button>
    <Drawer open={open} onClose={() => setOpen(false)} title="Examples">
      <Examples {...args} />
    </Drawer></>;
}
const meta = {
  title: "Workspace/Examples",
  component: Examples,
  render: args => <Preview {...args} />,
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

export const Loading: Story = {
  args:{selectedId:undefined,client:{...meta.args.client,list:()=>new Promise(()=>{})}},
};

export const ActionFailures: Story = {
  args:{client:{
    ...meta.args.client,
    list:async()=>({items:Array.from({length:16},(_,index)=>({
      id:`example-${index}`,sequence:index+1,name:`Saved example ${index+1}`,
      tool:'sample-lab',action:'mark_band',
    })),cursor:null}),
    open:async()=>{throw new Error('The saved inputs could not be read.');},
    save:async()=>{throw new Error('The example could not be saved.');},
  }},
};
export const EmptySaveFailure: Story = {
  args:{client:{...meta.args.client,
    list:async()=>({items:[],cursor:null}),
    save:async()=>{throw new Error('The example could not be saved.');},
  }},
};
