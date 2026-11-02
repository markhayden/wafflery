import { expect, test } from "bun:test";
import { parseDirectoryGrants } from "../../src/api/directory-grants";

test("directory observations keep requested and evidence permissions separate", () => {
  const value = { requested_dirs: ["/work/input"], evidence_dirs: ["/work/results"] };
  expect(parseDirectoryGrants(value)).toEqual(value);
  expect(parseDirectoryGrants({ requested_dirs: [], evidence_dirs: [] })).toEqual({ requested_dirs: [], evidence_dirs: [] });
  expect(parseDirectoryGrants(null)).toBeNull();
});

test("malformed grants are unavailable rather than made into empty or broader permissions", () => {
  for (const value of [undefined, {}, { roots: ["/work"] }, { requested_dirs: [], evidence_dirs: null }])
    expect(() => parseDirectoryGrants(value)).toThrow();
  for (const path of ["", ".", "work", "/work/../private", "/work/./input", "/work//input", "/work/", "/work\0/input", 123]) {
    for (const field of ["requested_dirs", "evidence_dirs"]) {
      expect(() => parseDirectoryGrants({requested_dirs: [], evidence_dirs: [], [field]: [path]})).toThrow();
    }
  }
  expect(() => parseDirectoryGrants({requested_dirs: Array(33).fill("/input"), evidence_dirs: []})).toThrow();
  expect(() => parseDirectoryGrants({requested_dirs: [], evidence_dirs: Array(33).fill("/output")})).toThrow();
  // Literal names are display data, not markup or shell commands. Root is an explicit valid grant.
  expect(parseDirectoryGrants({requested_dirs:["/", "/work/<literal> $(name)"], evidence_dirs:[]})).not.toBeNull();
});
