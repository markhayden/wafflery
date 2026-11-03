import {expect,test} from "bun:test";
import {artifactOccurrences} from "../../src/features/workspace/artifacts";
test("output occurrence order and duplicate identities remain independent",()=>{
 const entry={port:"masks",artifact:{type:"study:Mask",identity:"sha256:one",tool:"study",resource:"study://artifact/sha256:one"},preview:null};
 const found=artifactOccurrences({artifacts:{out:[entry,entry]}});
 expect(found).toHaveLength(2);expect(found.map(item=>item.index)).toEqual([0,1]);expect(artifactOccurrences(null)).toEqual([]);
});
