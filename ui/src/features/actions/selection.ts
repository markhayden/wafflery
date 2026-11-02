import type { Asset } from "../../api/assets";
import { encodeBindings, type ImageSlot } from "./model";

export type ImageFrame = {
  projection: "flat" | "equirectangular";
  topology: "bounded" | "wrap";
};
export type SelectedImage = Asset & { coordinates?: ImageFrame };
export type ImageSelection = Record<string, SelectedImage[]>;
const flat: ImageFrame = { projection: "flat", topology: "bounded" };

export function restoreSelection(inputs: Record<string, {asset: Asset; image: ImageFrame}[]>): ImageSelection {
  return Object.fromEntries(Object.entries(inputs).map(([slot, entries]) => [slot,
    entries.map(({asset, image}) => ({...asset, coordinates: {projection: image.projection, topology: image.topology}})),
  ]));
}

export function encodeSelection(version: string, slots: ImageSlot[], selected: ImageSelection) {
  const images = encodeBindings(version, slots, Object.fromEntries(
    Object.entries(selected).map(([slot, items]) => [slot, items.map(item => item.id)]),
  ));
  let totalBytes = 0;
  const frames: Record<string, ImageFrame[]> = {};
  for (const slot of Object.keys(images)) {
    frames[slot] = selected[slot]!.map(item => {
      if (item.size > 128 * 1024 * 1024) throw new Error("Each image must be at most 128 MiB.");
      totalBytes += item.size;
      if (totalBytes > 512 * 1024 * 1024) throw new Error("Attempt images must total at most 512 MiB.");
      return {...(item.coordinates ?? flat)};
    });
  }
  return {images, frames};
}
