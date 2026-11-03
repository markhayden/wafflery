import {expect,test} from "bun:test";
import {moveBinding,validatePorts} from "../../src/features/actions/ports";
import type {InputBinding} from "../../src/api/workspace";
test("port lists keep duplicate occurrence order and enforce scalar/required bounds",()=>{
 const a:InputBinding={kind:"artifact",reference:{identity:"a"}},b:InputBinding={kind:"artifact",reference:{identity:"b"}};
 expect(moveBinding([a,b,a],2,-1)).toEqual([a,a,b]);
 expect(validatePorts([{name:"mask",type:"Mask",required:true}],{})).toContain("mask");
 expect(validatePorts([{name:"mask",type:"Mask",required:true}],{mask:[a,b]})).toContain("one");
 expect(validatePorts([{name:"mask",type:"Mask",required:false,many:true}],{})).toBeUndefined();
});
