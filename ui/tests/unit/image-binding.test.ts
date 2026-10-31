import { expect, test } from "bun:test";
import { cardinalityError, moveImage, type Asset } from "../../src/api/assets";

test("single-image contracts reject multiple bindings and reordering preserves identities", () => {
  const a = { id: "a" } as Asset;
  const b = { id: "b" } as Asset;
  expect(cardinalityError(0, 1, 1)).not.toBeNull();
  expect(cardinalityError(1, 1, 1)).toBeNull();
  expect(cardinalityError(2, 1, 1)).not.toBeNull();
  const original = [a, b];
  expect(moveImage(original, 1, 0)).toEqual([b, a]);
  expect(original).toEqual([a, b]);
});
