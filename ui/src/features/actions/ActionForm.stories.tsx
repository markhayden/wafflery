import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { ActionForm } from "./ActionForm";
import type { Asset } from "../../api/assets";
import type { ImageSelection } from "./selection";
import sample from "../../../tests/fixtures/sample.png?url";
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
    upload: async (): Promise<Asset> => {
      throw new Error("Use ImageInput stories for uploads");
    },
    preview: async () => new Blob(),
  },
  render: function Form(args) {
    const [params, setParams] = useState(args.params);
    const [images, setImages] = useState<ImageSelection>(args.images);
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

const sampleAsset: Asset = {id:"a".repeat(32),sha256:"b".repeat(64),size:10000,width:160,height:100,media_type:"image/png"};
export const ImageLists: Story = {args:{
  action:{...meta.args.action,images:[{slot:"references",role:"reference",required:false,many:true}]},
  images:{references:[
    {...sampleAsset,coordinates:{projection:"flat",topology:"bounded"}},
    {...sampleAsset,coordinates:{projection:"equirectangular",topology:"wrap"}},
  ]},
  preview:async () => (await fetch(sample)).blob(),
  upload:async () => ({...sampleAsset}),
}};
