import type { Meta, StoryObj } from "@storybook/react-vite";
import type { Asset } from "../api/assets";
import { ImageThumb, ImageThumbGrid, ImageTile } from "./ImageThumb";

/** A generated picture so the story needs no fixture files. */
function swatch(hue: number): Promise<Blob> {
  const canvas = document.createElement("canvas");
  canvas.width = 160; canvas.height = 120;
  const context = canvas.getContext("2d")!;
  const gradient = context.createLinearGradient(0, 0, 160, 120);
  gradient.addColorStop(0, `hsl(${hue} 60% 55%)`); gradient.addColorStop(1, `hsl(${hue + 40} 50% 25%)`);
  context.fillStyle = gradient; context.fillRect(0, 0, 160, 120);
  return new Promise((resolve) => canvas.toBlob((blob) => resolve(blob!), "image/png"));
}
const asset = (id: string, width = 1024, height = 768): Asset => ({ id, sha256: id.padEnd(64, "0"), width, height, size: 1, media_type: "image/png" });
const preview = (value: Asset) => swatch(value.id.length * 47);

function Grid() {
  return <ImageThumbGrid>
    {["a", "bb", "ccc", "dddd"].map((id) => <ImageThumb key={id} asset={asset(id)} preview={preview} label={`Use image ${id}`} onPick={() => undefined} />)}
  </ImageThumbGrid>;
}
const meta = { title: "Controls/ImageThumb", component: Grid } satisfies Meta<typeof Grid>;
export default meta;
type Story = StoryObj<typeof meta>;
/** Recent images to pick from, each with its size. */
export const Thumbnails: Story = {};
/** The chosen image, shown small beside its details. */
export const Tile: Story = { render: () => <ImageTile asset={asset("tile")} preview={preview} /> };
