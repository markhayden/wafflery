import {expect,test} from "bun:test";
import {highlight} from "../../src/components/highlight";
test("highlighting never changes the text: the tokens join back to the input",()=>{
 const json=JSON.stringify({key:"a \"quoted\" value",n:-1.5e3,list:[true,false,null]},null,2);
 expect(highlight(json,"json").map(token=>token.text).join("")).toBe(json);
 const shell="cd '/tmp/a b' && '/bin/tool' 'call' 'x' '--arguments' '/tmp/a b/args.json'";
 expect(highlight(shell,"shell").map(token=>token.text).join("")).toBe(shell);
});
test("json keys, strings, numbers and literals; shell commands, flags and paths",()=>{
 const json=highlight('{"a": "b", "c": 2, "d": null}',"json").filter(token=>token.kind&&token.kind!=="punct").map(token=>`${token.kind}:${token.text}`);
 expect(json).toEqual(['key:"a"','string:"b"','key:"c"','number:2','key:"d"','literal:null']);
 const shell=highlight("cd '/tmp' && '/bin/tool' 'call' '--allow-dir' '/tmp'","shell").filter(token=>token.kind&&token.kind!=="punct").map(token=>`${token.kind}:${token.text}`);
 expect(shell).toEqual(["command:cd","path:'/tmp'","command:'/bin/tool'","string:'call'","flag:'--allow-dir'","path:'/tmp'"]);
});
