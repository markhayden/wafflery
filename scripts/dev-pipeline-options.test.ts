import { expect, test } from "bun:test";
import { pipelineOptions } from "./dev-pipeline-options";

test("offline demo supports attached start with optional browser opening", () => {
  expect(pipelineOptions([])).toEqual({ start: false, noOpen: false });
  expect(pipelineOptions(["--no-open"])).toEqual({ start: false, noOpen: true });
  expect(pipelineOptions(["--start"])).toEqual({ start: true, noOpen: false });
  expect(pipelineOptions(["--start", "--no-open"])).toEqual({ start: true, noOpen: true });
  expect(pipelineOptions(["--no-open", "--start"])).toEqual({ start: true, noOpen: true });
  for (const flags of [["--start", "--start"], ["--no-open", "--no-open"], ["--unknown"]]) {
    expect(() => pipelineOptions(flags)).toThrow();
  }
});
