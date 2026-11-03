import {expect,test} from "bun:test";
import type {RegisteredTool} from "../../src/api/workspace";
import {toolChoice} from "../../src/features/workspace/tool-presentation";
function tool(name:string,purpose:string,version="0.1.0",error:string|null=null){
 return {id:"registered-tool",error,declaration:{document:{tool:{name,purpose,version}}}} as RegisteredTool;
}
test("known presentation uses declared name and keeps the complete version unchanged",()=>{
 const rascal=toolChoice(tool("rascal","A contract purpose","0.1.1"));
 expect(rascal.id).toBe("registered-tool");expect(rascal.title).toBe("rascal");
 expect(rascal.icon).toContain("rascal.webp");expect(rascal.meta).toBe("Version 0.1.1");
 expect(rascal.description).toBe("Fully automated mask generation for precise image edits. Describe what to change and rascal finds the region, builds a high-quality mask and protects everything else, across a wide range of use cases.");
 const squint=toolChoice(tool("squint","A contract purpose","0.1.0 (6b9bfa327)"));
 expect(squint.icon).toContain("squint.webp");expect(squint.meta).toBe("Version 0.1.0 (6b9bfa327)");expect(squint.description).toBe("Image evaluation you can trust. Checks an image against the brief and references it was made from, and shows the evidence behind every verdict.");
});
test("unknown tools use purpose, including names that are object property names; errors win",()=>{
 for(const name of ["another-tool","constructor"]){const choice=toolChoice(tool(name,"Its declared purpose."));expect(choice.icon).toBeUndefined();expect(choice.description).toBe("Its declared purpose.");}
 expect(toolChoice(tool("rascal","Ignored","0.1.1","Cannot start this tool.")).description).toBe("Cannot start this tool.");
 expect(toolChoice({id:"rascal",declaration:null,error:"Invalid declaration."}).icon).toBeUndefined();
 expect(toolChoice({id:"broken",declaration:null,error:"Invalid declaration."})).toMatchObject({title:"broken",description:"Invalid declaration.",meta:"Version unknown"});
});
