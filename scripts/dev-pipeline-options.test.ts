import { expect, test } from "bun:test";
import { pipelineOptions } from "./dev-pipeline-options";

const plain = { start: false, noOpen: false, registerOnly: false };

test("offline demo supports attached start with optional browser opening", () => {
  expect(pipelineOptions([])).toEqual(plain);
  expect(pipelineOptions(["--no-open"])).toEqual({ ...plain, noOpen: true });
  expect(pipelineOptions(["--start"])).toEqual({ ...plain, start: true });
  expect(pipelineOptions(["--start", "--no-open"])).toEqual({ ...plain, start: true, noOpen: true });
  expect(pipelineOptions(["--no-open", "--start"])).toEqual({ ...plain, start: true, noOpen: true });
  for (const flags of [["--start", "--start"], ["--no-open", "--no-open"], ["--unknown"]]) {
    expect(() => pipelineOptions(flags)).toThrow();
  }
});

test("a persistent library can be registered without starting a host", () => {
  expect(pipelineOptions(["--library", "/tmp/dev", "--register-only"])).toEqual({
    ...plain, library: "/tmp/dev", registerOnly: true,
  });
  expect(pipelineOptions(["--start", "--library", "/tmp/dev"])).toEqual({ ...plain, start: true, library: "/tmp/dev" });
  for (const flags of [["--library"], ["--library", "relative"], ["--register-only", "--start"], ["--register-only", "--no-open"]]) {
    expect(() => pipelineOptions(flags)).toThrow();
  }
});
