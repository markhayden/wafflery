import { expect, test } from "bun:test";
import { parseToolSession, usd } from "../../src/api/tool-session";
const observation = {
  contract: "0.5.0", tool: { name: "fixture", version: "1" },
  id: "bb8f5520-a822-4cdc-a9eb-f1a72018e000", started: "2026-11-01T00:00:00Z",
  budget_usd: 2, spent_usd: 0.16, unresolved_usd: 0.04, reserved_usd: 0.04, remaining_usd: 1.8,
  allowed_dirs: ["/work/approved"],
};
test("session reading distinguishes an uncapped allowance from a zero balance", () => {
  expect(parseToolSession(observation).remaining_usd).toBe(1.8);
  expect(parseToolSession({ ...observation, budget_usd: null, remaining_usd: null }).remaining_usd).toBeNull();
  expect(() => parseToolSession({ ...observation, remaining_usd: null })).toThrow();
  expect(() => parseToolSession({ ...observation, spent_usd: undefined })).toThrow();
  expect(() => parseToolSession({ ...observation, remaining_usd: 2 })).toThrow();
  expect(() => parseToolSession({ ...observation, unresolved_usd: 1 })).toThrow();
  expect(() => parseToolSession({ ...observation, contract: "0.2.0" })).toThrow();
});
test("USD display preserves small reported charges", () => {
  expect(usd(0)).toBe("$0.00");
  expect(usd(0.00000001)).toBe("< $0.000001");
  expect(usd(0.000001)).toBe("$0.000001");
  expect(usd(1.8)).toBe("$1.80");
});

test("session access requires an explicit unique list of canonical absolute names", () => {
  expect(parseToolSession(observation)).toMatchObject({ allowed_dirs: ["/work/approved"] });
  expect(parseToolSession({ ...observation, allowed_dirs: [] })).toMatchObject({ allowed_dirs: [] });
  for (const allowed_dirs of [undefined, null, "/work", {}, [""], ["relative"], ["/work/"],
    ["/work/../private"], ["/work/./input"], ["/work//input"], ["/work\0/input"], [123], ["/work", "/work"]]) {
    expect(() => parseToolSession({ ...observation, allowed_dirs })).toThrow();
  }
  // Display is literal data. The host has already verified ownership and filesystem boundaries.
  expect(parseToolSession({ ...observation, allowed_dirs: ["/", "/work/<literal> $(name)"] }))
    .toMatchObject({ allowed_dirs: ["/", "/work/<literal> $(name)"] });
});
