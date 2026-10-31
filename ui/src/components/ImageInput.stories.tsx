import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { ImageInput } from "./ImageInput";
import type { Asset } from "../api/assets";
import sample from "../../tests/fixtures/sample.png?url";

const asset: Asset = { id: "a".repeat(32), sha256: "b".repeat(64), size: 25000, width: 160, height: 100, media_type: "image/png" };
const meta = {
  title: "Inputs/ImageInput",
  component: ImageInput,
  args: { label: "Reference image", value: [], min: 1, max: 1, onChange: () => {}, upload: async () => asset, preview: async () => (await fetch(sample)).blob() },
  render: function Controlled(args) {
    const [value, setValue] = useState<Asset[]>([...args.value]);
    return <ImageInput {...args} value={value} onChange={setValue} />;
  },
} satisfies Meta<typeof ImageInput>;
export default meta;
type Story = StoryObj<typeof meta>;

export const Empty: Story = {};
export const Selected: Story = { args: { value: [asset] } };
export const Multiple: Story = { args: { label: "Reference images", max: 8, value: [asset, { ...asset, id: "c".repeat(32), width: 320 }] } };
export const LongList: Story = { args: { label: "Reference images for a detailed visual comparison across the pipeline", max: 8, value: Array.from({ length: 8 }, (_, index) => ({ ...asset, id: String(index).repeat(32) })) } };
export const Disabled: Story = { args: { disabled: true, value: [asset] } };
export const Unavailable: Story = { args: { value: [asset], preview: async () => { throw new Error("Missing image"); } } };
export const UploadError: Story = { args: { upload: async () => { throw new Error("Not enough disk space to save this image."); } } };
