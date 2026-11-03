import {expect,test} from "bun:test";
import {artifactOccurrences} from "../../src/features/workspace/artifacts";
test("output occurrence order and duplicate identities remain independent",()=>{
 const entry={port:"masks",artifact:{type:"study:Mask",identity:"sha256:one",tool:"study",resource:"study://artifact/sha256:one"},preview:null};
 const found=artifactOccurrences({artifacts:{out:[entry,entry]}});
 expect(found).toHaveLength(2);expect(found.map(item=>item.index)).toEqual([0,1]);expect(artifactOccurrences(null)).toEqual([]);
});
import {compatibleDestinations,transferBinding} from "../../src/features/workspace/artifact-transfer";
test("native references stay with their tool and image transfer uses owned bytes",()=>{
 const source={reference:{type:"study:Mask",tool:"study",identity:"native"},asset:undefined};
 expect(compatibleDestinations(source,[{id:"other",declaration:{document:{tool:{name:"other"},actions:[{name:"op",kind:"operation",ports:{in:[{name:"mask",type:"study:Mask"}]}}]}}}] as never)).toEqual([]);
 expect(transferBinding({reference:{type:"image",tool:"study",identity:"sha256:abc"},asset:{id:"owned",sha256:"abc",width:10,height:5},frame:{projection:"flat",topology:"bounded"}},"other")).toEqual({kind:"image",asset:"owned",frame:{projection:"flat",topology:"bounded"}});
});
import {encodePortBindings} from "../../src/features/workspace/artifact-transfer";
test("lineage is separate from tool arguments and follows duplicate occurrence reordering",()=>{
 const first={kind:"artifact" as const,reference:{type:"study:Mask"},origin:{attempt:"a",port:"mask",occurrence:0}};
 const second={...first,origin:{...first.origin,occurrence:1}};
 const result=encodePortBindings({masks:[second,first,first]});
 expect(result.lineage.masks?.map(source=>source?.occurrence)).toEqual([1,0,0]);
 expect(result.bindings.masks?.every(item=>!("origin" in item))).toBe(true);
});
test("multiple compatible destinations remain explicit port choices",()=>{
 const source={reference:{type:"image",tool:"study",identity:"sha256:abc"},asset:{id:"owned",sha256:"abc",width:10,height:5}};
 const tools=[{id:"other",declaration:{document:{tool:{name:"other"},actions:[{name:"compare",title:"Compare",kind:"operation",ports:{in:[{name:"source",type:"image"},{name:"candidate",type:"image"}]}}]}}}];
 expect(compatibleDestinations(source,tools as never).map(item=>item.port)).toEqual(["source","candidate"]);
});
