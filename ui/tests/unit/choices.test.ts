import { expect, test } from "bun:test";
import { choiceStates, chosenChoices, hasChoiceRequirements, orderedChoices, readinessFor } from "../../src/features/actions/choices";
import type { Action } from "../../src/features/actions/model";

const field = { type: "string", enum: ["small", "large"], default: "large", "x-requirements": { small: ["model:small"], large: ["model:large"] } };
const action: Pick<Action, "readiness"> = { readiness: { state: "needs_setup", requirements: [
  { kind: "file", name: "model:small", state: "present", for_choice: { param: "model", value: "small" }, installable: "small" },
  { kind: "file", name: "model:large", state: "missing", for_choice: { param: "model", value: "large" }, installable: "large", setup: "Get the large model in the tool's setup." },
  { kind: "config", name: "settings", state: "present" },
] } };

test("choice states come only from requirements for that parameter's choices", () => {
  const states = choiceStates(action, "model");
  expect(states.get("small")).toEqual({ ready: true, installable: "small" });
  expect(states.get("large")).toEqual({ ready: false, installable: "large", setup: "Get the large model in the tool's setup." });
  expect(choiceStates(action, "other").size).toBe(0);
});

test("the default choice is listed first; a parameter without x-requirements is ordinary", () => {
  expect(orderedChoices(field)).toEqual(["large", "small"]);
  expect(hasChoiceRequirements(field)).toBe(true);
  expect(hasChoiceRequirements({ type: "string", enum: ["a", "b"] })).toBe(false);
});

test("a saved run names the choice it used, falling back to the declared default", () => {
  const declaration = { document: { actions: [{ name: "remove", params_schema: { properties: { model: { ...field, title: "Model" }, prompt: { type: "string" } } } }] } };
  expect(chosenChoices(declaration, "remove", { model: "small" })).toEqual([{ label: "Model", value: "small" }]);
  expect(chosenChoices(declaration, "remove", {})).toEqual([{ label: "Model", value: "large" }]);
  expect(chosenChoices(declaration, "other", {})).toEqual([]);
});


// remove_prompt's shape: settings everyone needs, one requirement per model choice, an optional key.
const remove: Pick<Action, "readiness" | "params_schema"> = {
  params_schema: { type: "object", required: ["prompt"], properties: {
    prompt: { type: "string" },
    filler: { type: "string", enum: ["lama", "bria"] },
    model: { ...field, default: "small" },
    projection: { type: "string", enum: ["flat", "equirectangular"] },
  } },
  readiness: { state: "ready", requirements: [
    { kind: "config", name: "rascal.toml", state: "present" },
    { kind: "file", name: "model:small", state: "present", for_choice: { param: "model", value: "small" } },
    { kind: "file", name: "model:large", state: "missing", for_choice: { param: "model", value: "large" }, setup: "Get the large model." },
    { kind: "credential", name: "CHECK_KEY", state: "missing", optional: true, for: "checking the result" },
  ] },
};
const names = (readiness: { requirements: { name: string }[] }) => readiness.requirements.map(item => item.name);

test("a requirement for a choice applies only when the inputs pick it, a blank input picking the default", () => {
  expect(names(readinessFor(remove, {}))).toEqual(["rascal.toml", "model:small", "CHECK_KEY"]);
  expect(names(readinessFor(remove, { model: "small" }))).toEqual(["rascal.toml", "model:small", "CHECK_KEY"]);
  expect(names(readinessFor(remove, { model: "large" }))).toEqual(["rascal.toml", "model:large", "CHECK_KEY"]);
});

test("readiness follows the chosen value: another choice's missing model never makes this one need setup", () => {
  expect(readinessFor(remove, {}).state).toBe("ready");
  expect(readinessFor(remove, { model: "large" }).state).toBe("needs_setup");
  // The tool computed needs_setup for its default; a call that picks an installed choice is ready.
  const missingDefault = { ...remove, readiness: { state: "needs_setup" as const, requirements: remove.readiness.requirements.map(item =>
    item.name === "model:small" ? { ...item, state: "missing" as const } : item.name === "model:large" ? { ...item, state: "present" as const } : item) } };
  expect(readinessFor(missingDefault, {}).state).toBe("needs_setup");
  expect(readinessFor(missingDefault, { model: "large" }).state).toBe("ready");
});

test("an optional requirement that is missing never makes the action unready", () => {
  const onlyOptional = { ...remove, readiness: { state: "ready" as const, requirements: [remove.readiness.requirements[3]!] } };
  expect(readinessFor(onlyOptional, {}).state).toBe("ready");
});

test("the tool's own state stands when nothing conditional is involved, and unavailable is never overridden", () => {
  const plain = { params_schema: {}, readiness: { state: "needs_setup" as const, requirements: [{ kind: "env" as const, name: "DIR", state: "present" as const }] } };
  expect(readinessFor(plain, {}).state).toBe("needs_setup");
  expect(readinessFor({ ...remove, readiness: { ...remove.readiness, state: "unavailable" } }, {}).state).toBe("unavailable");
});
