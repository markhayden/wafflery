import type { Meta, StoryObj } from "@storybook/react-vite";
import { PlaceMarks } from "./PlaceMarks";
import sample from "../../tests/fixtures/sample.png?url";
const asset = { id: "a".repeat(32), sha256: "b".repeat(64), size: 10063, width: 160, height: 100, media_type: "image/png" as const };
const image = { width: 160, height: 100, projection: "flat", topology: "bounded" };
const meta = {
  title: "Components/PlaceMarks", component: PlaceMarks,
  args: { asset, image, label: "Where the check still sees it · Result", load: async () => (await fetch(sample)).blob(),
    places: [{ kind: "bbox", parts: [{ bbox: [0.3, 0.38, 0.62, 0.62] }], label: "the band" }] },
} satisfies Meta<typeof PlaceMarks>;
export default meta;
type Story = StoryObj<typeof meta>;
export const OnePlace: Story = {};
export const Shapes: Story = { args: { places: [
  { kind: "bbox", parts: [{ bbox: [0.05, 0.1, 0.25, 0.4] }], label: "a seagull on the left lamp post" },
  { kind: "polygon", parts: [{ polygon: [[0.55, 0.15], [0.9, 0.2], [0.8, 0.55], [0.6, 0.45]] }], label: "a reflection" },
  { kind: "line", parts: [{ path: [[0.1, 0.85], [0.5, 0.7], [0.9, 0.85]], width_px: 3 }], label: "a seam" },
] } };
export const WithoutDescriptions: Story = { args: { places: [{ kind: "bbox", parts: [{ bbox: [0.4, 0.3, 0.7, 0.7] }] }] } };
export const Unavailable: Story = { args: { load: async () => { throw new Error("Missing bytes"); } } };
