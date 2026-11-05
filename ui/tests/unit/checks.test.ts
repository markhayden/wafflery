import { test, expect } from "bun:test";
import { blocking, checkView, imageName, requirementPurpose } from "../../src/features/workspace/checks";
import { outline } from "../../src/components/PlaceMarks";
import { isReadiness } from "../../src/api/workspace";
import { humanize } from "../../src/features/actions/help";
import type { SavedImage } from "../../src/api/workspace";
import type { Json, Requirement } from "../../src/features/actions/model";

const key: Requirement = { kind: "credential", name: "EXAMPLE_CHECK_KEY", state: "missing", optional: true, for: "checking the result" };
const config: Requirement = { kind: "config", name: "tool.toml", state: "missing" };
const frame = (sha: string) => ({ file_sha256: sha.repeat(64), width: 160, height: 100, projection: "flat", topology: "bounded" });
const saved = (role: string, sha: string, label?: string): SavedImage => ({
  role, ...(label ? { label } : {}), image: frame(sha),
  asset: { id: sha.repeat(32), sha256: sha.repeat(64), size: 1, width: 160, height: 100, media_type: "image/png" },
});
const outputs = [saved("result", "a", "Try 3 · result"), saved("result_try_1", "b", "Try 1 · result")];
const place = (sha: string, label = "the band") => ({ image: frame(sha), kind: "bbox", parts: [{ bbox: [0.3, 0.4, 0.6, 0.6] }], straddles_wrap: false, label });
const run = (extra: Record<string, Json>, notes: string[] = []): Json => ({ status: "completed", notes, ...extra });

test("an optional requirement says what it is for, and never blocks", () => {
  expect(requirementPurpose(key)).toBe("Needed for checking the result (optional)");
  expect(requirementPurpose({ kind: "env", name: "MODEL_DIR", state: "missing", optional: true })).toBe("Optional");
  expect(requirementPurpose(config)).toBeUndefined();
  expect(blocking([key, config])).toEqual([config]);
  expect(isReadiness({ state: "ready", requirements: [key] })).toBe(true);
  // `for` comes only with `optional` (table-manners 3.1).
  expect(isReadiness({ state: "ready", requirements: [{ ...config, for: "a part" }] })).toBe(false);
  expect(isReadiness({ state: "ready", requirements: [{ ...config, optional: "yes" }] })).toBe(false);
});

test("a pass says so, with the run's notes", () => {
  const view = checkView(run({ verdict: { value: "pass", where: [] } }, ["Kept try 1 of 1."]), outputs);
  expect(view).toEqual({ kind: "pass", title: "Check passed", notes: ["Kept try 1 of 1."] });
});

test("a fail or unsure says the check still sees it, and where", () => {
  const fail = checkView(run({ verdict: { value: "fail", where: [place("a")] } }), outputs);
  expect(fail?.kind).toBe("attention");
  expect(fail?.title).toBe("The check still sees it");
  if (fail?.kind !== "attention") throw new Error("expected attention");
  expect(fail.detail).toBe("Marked in 1 place on the result.");
  expect(fail.image?.role).toBe("result");
  expect(fail.places).toEqual([{ kind: "bbox", parts: [{ bbox: [0.3, 0.4, 0.6, 0.6] }], label: "the band" }]);
  const unsure = checkView(run({ verdict: { value: "unsure", where: [place("a"), place("a", "a shadow")] } }), outputs);
  if (unsure?.kind !== "attention") throw new Error("expected attention");
  expect(unsure.detail).toBe("The check was not sure. Marked in 2 places on the result.");
  const nowhere = checkView(run({ verdict: { value: "fail", where: [] } }), outputs);
  if (nowhere?.kind !== "attention") throw new Error("expected attention");
  expect(nowhere.detail).toBe("It did not say where.");
  // A place on an image this run did not keep is counted, never drawn on another image.
  const elsewhere = checkView(run({ verdict: { value: "fail", where: [place("c")] } }), outputs);
  if (elsewhere?.kind !== "attention") throw new Error("expected attention");
  expect(elsewhere.image).toBeUndefined();
  expect(elsewhere.detail).toBe("It named 1 place, on an image this run did not keep.");
});

test("a result that was not judged shows the tool's reason; anything else says nothing", () => {
  expect(checkView(run({}, ["Not checked: the checker needs ANTHROPIC_API_KEY."]), outputs))
    .toEqual({ kind: "unchecked", title: "Not checked: the checker needs ANTHROPIC_API_KEY." });
  expect(checkView(run({}, ["Kept try 1 of 2.", "The check could not run: the checker exited."]), outputs))
    .toEqual({ kind: "unchecked", title: "The check could not run: the checker exited." });
  expect(checkView(run({}, ["Illustrative offline test output."]), outputs)).toBeUndefined();
  expect(checkView({ status: "error", verdict: { value: "pass", where: [] }, notes: [] }, outputs)).toBeUndefined();
  expect(checkView(null, outputs)).toBeUndefined();
});

test("an image shows the tool's label, else its role in words", () => {
  expect(imageName(saved("mask_try_1", "b", "Try 1 · mask"), humanize)).toBe("Try 1 · mask");
  expect(imageName(saved("candidate", "b"), humanize)).toBe(humanize("candidate"));
});

test("places outline as their kind's shape, in fractions", () => {
  expect(outline("bbox", { bbox: [0.1, 0.2, 0.3, 0.4] })).toEqual([[0.1, 0.2], [0.3, 0.2], [0.3, 0.4], [0.1, 0.4]]);
  expect(outline("polygon", { polygon: [[0, 0], [1, 0], [1, 1]] })).toHaveLength(3);
  expect(outline("line", { column: 0.5, width_px: 2 })).toEqual([[0.5, 0], [0.5, 1]]);
  expect(outline("stroke", { path: [[0, 0], [1, 1]], radius_px: 3 })).toHaveLength(2);
  expect(outline("mask", { mask: {} })).toBeUndefined();
  expect(outline("bbox", { bbox: [0.1, 0.2] })).toBeUndefined();
});
