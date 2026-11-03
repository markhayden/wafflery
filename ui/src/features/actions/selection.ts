import type {BindingOrigin} from "../../api/workspace";
import type { Asset } from "../../api/assets";
import { encodeBindings, type ImageSlot } from "./model";

export type ImageFrame = {
  projection: "flat" | "equirectangular";
  topology: "bounded" | "wrap";
};
export type SelectedImage = Asset & { coordinates?: ImageFrame;origin?:BindingOrigin };
export type ImageSelection = Record<string, SelectedImage[]>;
const flat: ImageFrame = { projection: "flat", topology: "bounded" };

export function restoreSelection(inputs: Record<string, {asset: Asset; image: {projection:string;topology:string}}[]>,origins?:Record<string,(BindingOrigin|null)[]>): ImageSelection {
  return Object.fromEntries(Object.entries(inputs).map(([slot, entries]) => [slot,
    entries.map(({asset, image},index) => {
      const {projection, topology} = image;
      if ((projection !== "flat" && projection !== "equirectangular")
        || (topology !== "bounded" && topology !== "wrap")
        || (projection === "equirectangular" && topology !== "wrap"))
        throw new Error("Saved image coordinates are not supported.");
      return {...asset, coordinates: {projection, topology},...(origins?.[slot]?.[index]?{origin:origins[slot]![index]!}:{})};
    }),
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
  const lineage:Record<string,(BindingOrigin|null)[]>={};
  for(const slot of Object.keys(images)){if(selected[slot]!.some(image=>image.origin))lineage[slot]=selected[slot]!.map(image=>image.origin??null);}
  return {images, frames,...(Object.keys(lineage).length?{lineage}:{})};
}
