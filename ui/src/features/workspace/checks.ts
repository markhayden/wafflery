/** Plain words for what a tool reports about checking a result: optional requirements
 * (table-manners 3.1), a judged result's verdict (2.4) and the notes of a result that was not
 * judged. Wafflery holds no logic about checkers; it says what the tool said. */
import type { SavedImage } from "../../api/workspace";
import type { Place } from "../../components/PlaceMarks";
import { object, type Json, type Requirement } from "../actions/model";

/** What an optional requirement is for ("Needed for checking the result (optional)"); nothing
 * for a requirement the action cannot run without. */
export function requirementPurpose(requirement: Requirement): string | undefined {
  if (!requirement.optional) return undefined;
  return requirement.for ? `Needed for ${requirement.for} (optional)` : "Optional";
}

/** The requirements that can hold an action back: an optional one never does. */
export function blocking(requirements: Requirement[]): Requirement[] {
  return requirements.filter((requirement) => !requirement.optional);
}

export type CheckView =
  | { kind: "pass"; title: string; notes: string[] }
  | { kind: "attention"; title: string; detail: string; places: Place[]; image?: SavedImage; notes: string[] }
  | { kind: "unchecked"; title: string };

const NOT_JUDGED = ["Not checked: ", "The check could not run: "];
const count = (n: number) => `${n} ${n === 1 ? "place" : "places"}`;

function place(value: Json): Place | undefined {
  const location = object(value);
  if (!location || typeof location.kind !== "string" || !Array.isArray(location.parts)) return undefined;
  return { kind: location.kind, parts: location.parts, ...(typeof location.label === "string" ? { label: location.label } : {}) };
}

/** What to say about a run's check, or nothing when the result neither carries a verdict nor
 * says why it was not judged. */
export function checkView(result: Json | undefined, outputs: SavedImage[]): CheckView | undefined {
  const run = object(result);
  if (!run || run.status !== "completed") return undefined;
  const notes = Array.isArray(run.notes) ? run.notes.filter((note): note is string => typeof note === "string") : [];
  const verdict = object(run.verdict);
  if (!verdict) {
    const note = notes.find((text) => NOT_JUDGED.some((start) => text.startsWith(start)));
    return note ? { kind: "unchecked", title: note } : undefined;
  }
  if (verdict.value === "pass") return { kind: "pass", title: "Check passed", notes };
  if (verdict.value !== "fail" && verdict.value !== "unsure") return undefined;
  const where = Array.isArray(verdict.where) ? verdict.where : [];
  const places = where.flatMap((value) => place(value) ?? []);
  // The places are in the frame of the image the check judged: draw them on that saved image.
  const judged = object(object(where[0])?.image)?.file_sha256;
  const image = typeof judged === "string" ? outputs.find((output) =>
    (output.image as { file_sha256?: unknown }).file_sha256 === judged || output.asset.sha256 === judged) : undefined;
  const unsure = verdict.value === "unsure" ? "The check was not sure. " : "";
  const detail = places.length === 0 ? `${unsure}It did not say where.`
    : image ? `${unsure}Marked in ${count(places.length)} on the result.`
      : `${unsure}It named ${count(places.length)}, on an image this run did not keep.`;
  return { kind: "attention", title: "The check still sees it", detail, places, ...(image ? { image } : {}), notes };
}

/** The name a saved image shows: the tool's label when it gave one, else its role in words. */
export function imageName(image: Pick<SavedImage, "role" | "label">, humanize: (role: string) => string): string {
  return image.label ?? humanize(image.role);
}
