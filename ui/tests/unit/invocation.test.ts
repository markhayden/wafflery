import { expect, test } from "bun:test";
import { savedInvocation } from "../../src/features/workspace/invocation";
import type { Json } from "../../src/features/actions/model";

const invocation = {
  action: "soften_token",
  arguments: { amount: 0.25, max_spend_usd: 0.1, options: { label: "saved" } },
  effective: { model: "fixture-model", config_identity: `sha256:${"1".repeat(64)}` },
  requirements: ["TEST_MODEL_KEY", "config.toml"],
};

test("resolved invocation stays exact across all terminal outcomes", () => {
  for (const status of ["completed", "cancelled", "error", "invalid"]) {
    const result = { contract: "0.6.0", status, invocation: structuredClone(invocation) };
    const before = JSON.stringify(result);
    expect(savedInvocation(result)).toEqual({ state: "resolved", value: invocation });
    expect(JSON.stringify(result)).toBe(before);
  }
});

test("only explicit null on an invalid run means inputs did not resolve", () => {
  expect(savedInvocation({ contract: "0.6.0", status: "invalid", invocation: null }))
    .toEqual({ state: "unresolved" });
  for (const result of [null, {}, { contract: "0.6.0", status: "invalid" },
    { contract: "0.6.0", status: "completed", invocation: null },
    { contract: "0.3.0", status: "invalid", invocation: null }]) {
    expect(savedInvocation(result)).toEqual({ state: "unavailable" });
  }
});

test("older or malformed evidence never becomes a reconstructed invocation", () => {
  for (const version of ["0.1.0", "0.2.0", "0.3.0"]) {
    expect(savedInvocation({ contract: version, invocation })).toEqual({ state: "unavailable" });
  }
  for (const malformed of [[], "unknown", {}, { ...invocation, arguments: [] },
    { ...invocation, effective: null }, { ...invocation, requirements: [{ value: "not a name" }] }]) {
    expect(savedInvocation({ contract: "0.6.0", status: "completed", invocation: malformed } as Json))
      .toEqual({ state: "unavailable" });
  }
});
