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
      kind: "task",
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

export const PaidJson: Story = {args:{
  action:{...meta.args.action,cost:{may_spend:true},params_schema:{
    type:"object",allOf:[{required:["prompt"]}],properties:{prompt:{type:"string"},max_spend_usd:{type:"number",exclusiveMinimum:0}},
  }},
  params:{prompt:"Example",max_spend_usd:0.5},
}};
/** A setting typed as JSON with the tool's example (rascal's `rect`), beside a choice. */
export const JsonSetting: Story = { args: { action: { kind: "operation", name: "rect", title: "Rect", purpose: "A rectangle as a location.",
  cost: { may_spend: false }, readiness: { state: "ready", requirements: [] },
  params_schema: { type: "object", required: ["bbox"], properties: {
    bbox: { type: "array", items: { type: "number" }, minItems: 4, maxItems: 4, description: "[x0, y0, x1, y1] as fractions of the frame; x0 < x1 and y0 < y1", examples: [[0.25, 0.25, 0.75, 0.75]] },
    mode: { type: "string", enum: ["flat", "equirectangular"], default: "flat", description: "How the image wraps." } } } }, params: {} } };
/** Settings of which exactly one is given (rascal's `line`: column or path) as a required choice. */
export const ExactlyOneOf: Story = { args: { action: { kind: "operation", name: "line", title: "Line", purpose: "A line location.",
  cost: { may_spend: false }, readiness: { state: "ready", requirements: [] },
  params_schema: { type: "object", required: ["width_px"], oneOf: [{ required: ["column"] }, { required: ["path"] }], properties: {
    column: { type: "number", minimum: 0, maximum: 1, description: "The column as a fraction of the width, for a whole-column line." },
    path: { type: "array", minItems: 2, items: { type: "array", items: { type: "number" }, minItems: 2, maxItems: 2 }, description: "Two or more [x, y] points as fractions of the frame, for a path.", examples: [[[0.2, 0.5], [0.8, 0.5]]] },
    width_px: { type: "number", exclusiveMinimum: 0, description: "The line's full width in pixels." } } } }, params: {} } };
