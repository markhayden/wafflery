// Plain-language labels and help for an action's inputs, for someone new to the tools.
// Images and locations are the contract's own concepts (table-manners 8, 9), so their help is
// written here once for every tool. Settings come from the tool's JSON Schema: its `title` and
// `description` when it gives them, a readable form of the key when it does not.
import type { Schema } from "./model";

const units: Record<string, string> = { usd: "USD", px: "px", s: "seconds", ms: "ms", pct: "%" };

/** `max_spend_usd` → "Max spend (USD)"; `grow-px` → "Grow (px)". */
export function humanize(name: string): string {
  const words = name.split(/[_\-\s.]+/).filter(Boolean);
  const last = words.at(-1)?.toLowerCase();
  const unit = last && words.length > 1 ? units[last] : undefined;
  const text = (unit ? words.slice(0, -1) : words).join(" ").toLowerCase();
  const label = text.charAt(0).toUpperCase() + text.slice(1);
  return unit ? `${label} (${unit})` : label;
}

/** An image slot: "Source image", or "Before (reference image)" when the slot names its own job. */
export function imageLabel(slot: { slot: string; role: string }): string {
  if (slot.slot === slot.role || slot.slot === "image") return `${humanize(slot.role)} image`;
  return `${humanize(slot.slot)} (${slot.role} image)`;
}

const roleHelp: Record<string, string> = {
  source: "The image the action works on. The tool reads it and makes its result from it; your file is never changed.",
  reference: "An image to compare against or take details from. The tool reads it but does not change it.",
  candidate: "The image being checked, usually one a generator made from your brief.",
  mask: "A black-and-white image marking an area: white is inside, black is outside.",
};

export function imageHelp(slot: { slot: string; role: string; many?: boolean }): string {
  const job = roleHelp[slot.role] ?? `An image the action uses as its ${slot.role}.`;
  return `${job} Choose an image you saved before, or import a PNG or JPEG from your computer. `
    + `If it is a 360° panorama, set its coordinates to Panorama so edges that meet at the wrap are treated as one.`
    + (slot.many ? " You can add several; their order is kept." : "");
}

const kindNames: Record<string, string> = {
  bbox: "a rectangle", polygon: "a shape", line: "a line", stroke: "a brush stroke",
  mask: "a mask image", whole_image: "the whole image", view: "a panorama view", absent: "nothing found",
};

/** A location slot: "Area (a rectangle)", or the slot's own name. */
export function locationLabel(slot: string, kinds: string[] = []): string {
  const name = slot === "location" ? "Area" : humanize(slot);
  const shapes = kinds.map((kind) => kindNames[kind]).filter(Boolean);
  return shapes.length ? `${name} (${shapes.join(" or ")})` : name;
}

export function locationHelp(kinds: string[] = []): string {
  const base = "A location marks where on the image the action should work. It is measured on the original image, so it stays right at any zoom.";
  const how = kinds.includes("bbox")
    ? " Draw it: press on the image and drag a rectangle over the area. You can then type exact pixel values."
    : kinds.includes("stroke")
      ? " Paint it: press on the image and drag along the damage, with a brush just wide enough to cover it. Several strokes are fine."
      : " Paste it as JSON, for example a location another tool produced.";
  return base + how;
}

export function paramLabel(key: string, field: Schema): string {
  return typeof field.title === "string" ? field.title : humanize(key);
}

/** The tool's description, then what the field accepts, in words. */
export function paramHelp(field: Schema, required: boolean): string {
  const parts: string[] = [];
  if (typeof field.description === "string") parts.push(field.description);
  if (Array.isArray(field.enum)) parts.push(`Choices: ${field.enum.map(String).join(", ")}.`);
  if (field.type === "boolean") parts.push("Yes or no.");
  if (field.type === "number" || field.type === "integer") {
    const bounds = [
      typeof field.minimum === "number" ? `at least ${field.minimum}` : undefined,
      typeof field.exclusiveMinimum === "number" ? `above ${field.exclusiveMinimum}` : undefined,
      typeof field.maximum === "number" ? `at most ${field.maximum}` : undefined,
    ].filter(Boolean);
    parts.push(`${field.type === "integer" ? "A whole number" : "A number"}${bounds.length ? `, ${bounds.join(" and ")}` : ""}.`);
  }
  if (field.default !== undefined) parts.push(`Default: ${String(field.default)}.`);
  parts.push(required ? "Required." : "Optional: leave it empty to use the tool's default.");
  return parts.join(" ");
}
