import { object, type Action, type Json, type Requirement, type Schema } from "./model";

/** Where a choice stands (table-manners 0.6 draft, 3.1): ready unless one of its requirements is
 * missing or invalid; the installable that would satisfy it, and the tool's setup line. */
export type ChoiceState = { ready: boolean; installable?: string; setup?: string };

/** True when this parameter's choices carry their own requirements (`x-requirements`). */
export function hasChoiceRequirements(field: Schema): boolean {
  return Array.isArray(field.enum) && !!object(field["x-requirements"]);
}
/** Each choice's state, from the action's readiness requirements with `for_choice`. */
export function choiceStates(action: Pick<Action, "readiness">, key: string): Map<string, ChoiceState> {
  const states = new Map<string, ChoiceState>();
  for (const requirement of action.readiness.requirements as Requirement[]) {
    const choice = requirement.for_choice;
    if (!choice || choice.param !== key) continue;
    const current = states.get(choice.value) ?? { ready: true };
    const missing = requirement.state === "missing" || requirement.state === "invalid";
    const next: ChoiceState = { ready: current.ready && !missing };
    const installable = current.installable ?? requirement.installable;
    const setup = current.setup ?? (missing ? requirement.setup : undefined);
    if (installable !== undefined) next.installable = installable;
    if (setup !== undefined) next.setup = setup;
    states.set(choice.value, next);
  }
  return states;
}
/** The choices in the order a person reads them: the default first, then as the tool lists them. */
export function orderedChoices(field: Schema): Json[] {
  const choices = Array.isArray(field.enum) ? [...field.enum] : [];
  const index = field.default === undefined ? -1 : choices.findIndex(choice => choice === field.default);
  if (index > 0) choices.unshift(...choices.splice(index, 1));
  return choices;
}
/** For a saved run: each choice-with-requirements parameter and the value the run used (the
 * request's, else the declared default), so a result can say which model made it. */
export function chosenChoices(declaration: unknown, actionName: string, params: unknown): { label: string; value: string }[] {
  const document = object(object(declaration as Json)?.document);
  const actions = Array.isArray(document?.actions) ? document.actions : [];
  const action = actions.map(object).find(entry => entry?.name === actionName);
  const properties = object(object(action?.params_schema)?.properties) ?? {};
  const given = object(params as Json) ?? {};
  return Object.entries(properties).flatMap(([key, entry]) => {
    const field = object(entry);
    if (!field || !hasChoiceRequirements(field)) return [];
    const value = given[key] ?? field.default;
    if (value === undefined || value === null) return [];
    return [{ label: typeof field.title === "string" ? field.title : key, value: String(value) }];
  });
}
