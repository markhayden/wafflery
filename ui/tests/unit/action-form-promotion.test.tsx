import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { ActionForm } from "../../src/features/actions/ActionForm";
import type { Action, Schema } from "../../src/features/actions/model";

// remove_prompt's shape: a required prompt, an optional filler, optional settings, and an optional
// model choice that the readiness depends on.
const action: Action = {
  kind: "task", name: "remove_prompt", title: "Remove what a prompt names", purpose: "Remove it.",
  params_schema: { type: "object", required: ["prompt"], properties: {
    check: { type: "boolean", title: "Check the result", default: true },
    filler: { type: "string", title: "Filler", enum: ["lama", "bria"] },
    model: { type: "string", title: "Model", enum: ["tiny", "birefnet"], default: "tiny", "x-requirements": { tiny: ["segmenter:tiny"], birefnet: ["segmenter:birefnet"] } },
    projection: { type: "string", title: "Projection", enum: ["flat", "equirectangular"] },
    prompt: { type: "string", title: "Prompt" },
  } },
  cost: { may_spend: false },
  readiness: { state: "ready", requirements: [
    { kind: "model", name: "segmenter:tiny", state: "present", for_choice: { param: "model", value: "tiny" } },
    { kind: "model", name: "segmenter:birefnet", state: "present", for_choice: { param: "model", value: "birefnet" } },
  ] },
};
const render = (params: Schema) => renderToStaticMarkup(<ActionForm action={action} params={params} onChange={() => undefined}
  images={{}} onImages={() => undefined} upload={async () => { throw new Error("unused"); }} preview={async () => new Blob()} />);
const labels = (html: string) => [...html.matchAll(/<span id="[^"]+-label">([^<]+)<\/span>/g)].map(match => match[1]);

test("a choice the readiness depends on sits with the required settings, before the other optional ones", () => {
  expect(labels(render({}))).toEqual(["Prompt", "Model", "Check the result", "Filler", "Projection"]);
});

test("it shows the choice a run will use: the default when the input is blank, never 'Not set'", () => {
  const blank = render({});
  const model = blank.slice(blank.indexOf("Model</span>"), blank.indexOf("Check the result</span>"));
  expect(model).not.toContain("Not set");
  expect(model).toMatch(/<option value="&quot;tiny&quot;" selected="">tiny<\/option>/);
  const picked = render({ model: "birefnet" });
  expect(picked.slice(picked.indexOf("Model</span>"))).toMatch(/<option value="&quot;birefnet&quot;" selected="">birefnet<\/option>/);
  // An ordinary optional choice still offers leaving it unset.
  expect(blank.slice(blank.indexOf("Filler</span>"))).toContain("Not set");
});
