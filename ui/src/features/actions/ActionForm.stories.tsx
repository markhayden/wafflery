import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { ActionForm } from "./ActionForm";
import type { Asset } from "../../api/assets";
import { Button } from "../../components/Button";
const meta = {
  title: "Inputs/ActionForm",
  component: ActionForm,
  args: {
    action: {
      name: "fixture",
      title: "Fixture",
      purpose: "Declared fields",
      cost: { may_spend: false },
      readiness: { state: "ready", requirements: [] },
      params_schema: {
        type: "object",
        required: ["prompt"],
        properties: {
          prompt: { type: "string" },
          enabled: { type: "boolean" },
          tags: { type: "array", items: { type: "string" } },
        },
      },
    },
    params: {},
    images: {},
    onChange: () => {},
    onImages: () => {},
    upload: async () => {
      throw new Error("Use ImageInput stories for uploads");
    },
    preview: async () => new Blob(),
  },
  render: function Form(args) {
    const [params, setParams] = useState(args.params);
    const [images, setImages] = useState<Record<string, Asset[]>>({});
    const [saved, setSaved] = useState("");
    return (
      <form
        onSubmit={(event) => {
          event.preventDefault();
          setSaved(JSON.stringify(params));
        }}
      >
        <ActionForm
          {...args}
          params={params}
          onChange={setParams}
          images={images}
          onImages={(slot, value) => setImages({ ...images, [slot]: value })}
        />
        <Button type="submit">Validate form</Button>
        <output>{saved}</output>
      </form>
    );
  },
} satisfies Meta<typeof ActionForm>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Declared: Story = {};
export const Disabled: Story = { args: { disabled: true } };
