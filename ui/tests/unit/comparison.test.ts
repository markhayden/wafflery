import {test,expect} from "bun:test";
import {canWipe} from "../../src/components/comparison";
test("wipe requires equal dimensions and coordinate systems, with explicit missing images",()=>{
 const frame={width:160,height:100,projection:"flat",topology:"bounded"};
 expect(canWipe(frame,frame)).toBe(true);
 expect(canWipe(frame,{...frame,width:320})).toBe(false);
 expect(canWipe(frame,{...frame,topology:"wrap_x"})).toBe(false);
 expect(canWipe(frame,undefined)).toBe(false);
});
