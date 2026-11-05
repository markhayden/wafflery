import { expect, test } from "bun:test";
import { encodeSelection, restoreSelection } from "../../src/features/actions/selection";
import { moveImage, type Asset } from "../../src/api/assets";

const asset = {id:"a",size:100} as Asset;
const flat = {projection:"flat",topology:"bounded"} as const;
const wrap = {projection:"equirectangular",topology:"wrap"} as const;
const slots = [{slot:"references",role:"reference",required:false,many:true}];

test("saved occurrences retain their own coordinates through reorder and removal", () => {
  const saved = {references:[{asset,image:flat},{asset,image:wrap}]};
  const restored = restoreSelection(saved);
  const reordered = moveImage(restored.references!,1,0);
  expect(encodeSelection("0.5.0", slots, {references:reordered})).toEqual({
    images:{references:["a","a"]},frames:{references:[wrap,flat]},
  });
  expect(encodeSelection("0.5.0", slots, {references:reordered.slice(1)}).frames).toEqual({references:[flat]});
  expect(saved.references[0]!.image).toEqual(flat);
});

test("empty optional selections omit frames and images; new images default to flat", () => {
  expect(encodeSelection("0.5.0", slots, {references:[]})).toEqual({images:{},frames:{}});
  expect(encodeSelection("0.5.0", slots, {references:[asset]}).frames).toEqual({references:[flat]});
});

test("duplicate occurrences count toward the aggregate byte limit", () => {
  const large = {...asset,size:128*1024*1024};
  expect(() => encodeSelection("0.5.0", slots, {references:Array(5).fill(large)})).toThrow("512 MiB");
  expect(() => encodeSelection("0.5.0", slots, {references:[{...large,size:large.size+1}]})).toThrow("128 MiB");
  expect(encodeSelection("0.5.0", slots, {references:Array(4).fill(large)}).images.references).toHaveLength(4);
});

test("unsupported saved coordinates are refused rather than silently reinterpreted", () => {
  expect(() => restoreSelection({references:[{asset,image:{projection:"cylindrical",topology:"wrap"} as never}]})).toThrow("Saved image coordinates");
  expect(() => restoreSelection({references:[{asset,image:{projection:"equirectangular",topology:"bounded"}}]})).toThrow("Saved image coordinates");
});
