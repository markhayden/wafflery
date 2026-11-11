import { expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { Requirements } from "../../src/features/workspace/ToolSetup";
import type { Requirement } from "../../src/features/actions/model";

const present: Requirement[] = [
  { kind: "config", name: "rascal.toml", state: "present" },
  { kind: "env", name: "RASCAL_TOOLS_MODELS", state: "present" },
  { kind: "model", name: "segmenter:tiny", state: "present", for_choice: { param: "model", value: "tiny" } },
];
const optionalKey: Requirement = { kind: "credential", name: "ANTHROPIC_API_KEY", state: "missing", optional: true, for: "checking the result" };
const html = (requirements: Requirement[]) => renderToStaticMarkup(<Requirements requirements={requirements} />);

test("nothing missing: every requirement sits behind one closed disclosure, with its count", () => {
  const ready = html([...present, optionalKey]);
  expect(ready).toContain("<details");
  expect(ready).not.toContain("<details open");
  expect(ready).toContain("Show requirements (4)");
  expect(ready).not.toContain("Missing requirements");
});

test("something required missing: the missing ones are listed open, the rest stay collapsed", () => {
  const missing: Requirement = { kind: "model", name: "segmenter:birefnet", state: "missing", for_choice: { param: "model", value: "birefnet" }, setup: "Get it in setup." };
  const view = html([missing, ...present]);
  const open = view.slice(0, view.indexOf("<details"));
  expect(open).toContain('aria-label="Missing requirements"');
  expect(open).toContain("segmenter:birefnet");
  expect(open).toContain("Get it in setup.");
  expect(open).toContain("Needed because model is birefnet.");
  expect(open).not.toContain("rascal.toml");
  expect(view).toContain("Show other requirements (3)");
});

test("a missing optional requirement reads as optional and is never listed as needed", () => {
  const view = html([optionalKey]);
  expect(view).not.toContain("Missing requirements");
  expect(view).toContain("Missing · optional");
  expect(view).toContain("Needed for checking the result (optional)");
});

test("'not validated' is said once, not on every present line", () => {
  const view = html(present);
  expect(view.match(/Present</g)?.length).toBe(3);
  expect(view).not.toContain("Present · not validated");
  expect(view.match(/has not checked that it works/g)?.length).toBe(1);
});
