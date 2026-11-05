import { expect, test } from "bun:test";
import { parseRoute, routePath } from "../../src/features/workspace/route";

test("an address names a tool, then an action, then a run, each optional from the right", () => {
  expect(parseRoute("/")).toEqual({});
  expect(parseRoute("/rascal")).toEqual({ tool: "rascal" });
  expect(parseRoute("/rascal/remove")).toEqual({ tool: "rascal", action: "remove" });
  expect(parseRoute("/rascal/remove/77448f00-9518")).toEqual({ tool: "rascal", action: "remove", run: "77448f00-9518" });
  expect(parseRoute("/rascal/remove/")).toEqual({ tool: "rascal", action: "remove" });
});

test("a place round-trips through its address, and a part without its parent is dropped", () => {
  for (const route of [{}, { tool: "squint" }, { tool: "rascal", action: "mask_rect" }, { tool: "rascal", action: "remove", run: "a b" }])
    expect(parseRoute(routePath(route))).toEqual(route);
  expect(routePath({ tool: "rascal", run: "x" })).toBe("/rascal");
  expect(routePath({ tool: "", action: "remove" })).toBe("/");
});

test("a malformed address is the empty place, not an error", () => {
  expect(parseRoute("/%E0%A4%A")).toEqual({});
});
