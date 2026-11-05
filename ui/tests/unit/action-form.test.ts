import { describe, expect, test } from "bun:test";
import { initialParameters, encodeBindings, fieldMode } from "../../src/features/actions/model";
describe("declared action inputs", () => {
  test("uses declared defaults without inventing optional parameters", () => {
    expect(initialParameters({ type: "object", properties: { count: {type:"integer",default:2}, prompt:{type:"string"} }})).toEqual({count:2});
  });
  test("keeps complex and array schemas in an explicit JSON editor", () => {
    expect(fieldMode({type:"array",items:{type:"string"}})).toBe("json");
    expect(fieldMode({anyOf:[{type:"string"},{type:"number"}]})).toBe("json");
    expect(fieldMode({type:"string",enum:["a","b"]})).toBe("enum");
  });
  test("encodes scalar v0.3 image slots from ordered arrays and refuses cardinality drift", () => {
    expect(encodeBindings("0.5.0", [{slot:"source",role:"source",required:true}], {source:["asset-a"]})).toEqual({source:["asset-a"]});
    expect(() => encodeBindings("0.5.0", [{slot:"source",role:"source",required:true}], {source:[]})).toThrow("requires one");
    expect(() => encodeBindings("0.5.0", [{slot:"source",role:"source",required:false}], {source:["a","b"]})).toThrow("at most one");
    expect(() => encodeBindings("0.1.0", [], {})).toThrow("Unsupported");
    expect(() => encodeBindings("0.3.0", [], {})).toThrow("Unsupported");
  });
});

test("encodes ordered lists with duplicates and omits empty optional slots", () => {
  const slots = [{slot:"references", role:"reference", required:false, many:true}];
  expect(encodeBindings("0.5.0", slots, {references:["b","a","b"]})).toEqual({references:["b","a","b"]});
  expect(encodeBindings("0.5.0", slots, {references:[]})).toEqual({});
  expect(() => encodeBindings("0.5.0", [{...slots[0]!, required:true}], {references:[]})).toThrow("at least one");
  expect(() => encodeBindings("0.5.0", slots, {references:Array(33).fill("a")})).toThrow("32 images");
});
