import { expect, test } from "bun:test";
import { choiceStates, chosenChoices, hasChoiceRequirements, orderedChoices } from "../../src/features/actions/choices";
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

