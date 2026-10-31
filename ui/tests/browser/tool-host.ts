import {test as base,expect,type Page} from "@playwright/test";
import {spawn,spawnSync,type ChildProcess} from "node:child_process";
import {mkdtemp,mkdir,readFile,realpath,rm,writeFile} from "node:fs/promises";
import {tmpdir} from "node:os";
import {join,resolve} from "node:path";
import {createConnection} from "node:net";
export type Host={root:string;origin:string;open:(page:Page)=>Promise<void>;stop:()=>Promise<void>;start:()=>Promise<void>};
export const test=base.extend<{host:Host;hostScenario:string}>({hostScenario:["normal",{option:true}],host:async({hostScenario},use)=>{
 const build=spawnSync("cargo",["build","--locked","--example","sample-lab","--bin","wafflehouse"],{cwd:resolve(".."),encoding:"utf8"});if(build.status!==0)throw new Error(build.stderr);
 const temporary=await mkdtemp(join(tmpdir(),"wafflehouse-workflow-"));const root=join(temporary,"data"),toolRoot=join(temporary,"tool");await mkdir(root,{mode:0o700});await mkdir(toolRoot,{mode:0o700});
 await writeFile(join(root,"tools.json"),JSON.stringify([{id:"sample",executable:resolve("../target/debug/examples/sample-lab"),args:[toolRoot,hostScenario],cwd:toolRoot,roots:[toolRoot]}]));
 let child:ChildProcess;let origin="";
 async function control(operation:string):Promise<{bootstrap:string}>{const canonical=await realpath(root);const discovery=JSON.parse(await readFile(join(root,"host.json"),"utf8")) as {socket:string};return new Promise((resolveReply,reject)=>{const socket=createConnection(discovery.socket);let response="";socket.setTimeout(2000,()=>socket.destroy(new Error("control timeout")));socket.on("error",reject);socket.on("connect",()=>socket.end(JSON.stringify({protocol:1,root:canonical,operation})));socket.on("data",chunk=>{response+=String(chunk);});socket.on("end",()=>{try{resolveReply(JSON.parse(response) as {bootstrap:string});}catch(e){reject(e);}});});}
 const host:Host={root,get origin(){return origin;},async start(){child=spawn(resolve("../target/debug/wafflehouse"),["--foreground","--no-open","--data-dir",root],{stdio:["ignore","pipe","pipe"]});origin=await new Promise((resolveOrigin,reject)=>{child.once("error",reject);child.once("exit",()=>reject(new Error("Host exited before readiness")));child.stdout?.once("data",chunk=>resolveOrigin(String(chunk).trim()));});await expect.poll(async()=>{try{await control("status");return true;}catch{return false;}}).toBe(true);},async stop(){if(child&&child.exitCode===null){const exited=new Promise<void>(resolveExit=>child.once("exit",()=>resolveExit()));child.kill("SIGTERM");
 await Promise.race([exited,new Promise(resolve=>setTimeout(resolve,250))]);
 if(child.exitCode===null){
  try{
   const {bootstrap}=await control("bootstrap");
   const session=await(await fetch(`${origin}/api/session`,{method:"POST",headers:{Origin:origin,"Content-Type":"application/json"},body:JSON.stringify({bootstrap})})).json() as {token:string};
   const headers={Origin:origin,Authorization:`Bearer ${session.token}`,"Content-Type":"application/json"};
   for(let tries=0;tries<30&&child.exitCode===null;tries++){
    const state=await(await fetch(`${origin}/api/host/shutdown`,{headers})).json() as {phase:string};
    if(state.phase==="waiting"){await fetch(`${origin}/api/host/force-quit`,{method:"POST",headers,body:"{}"});break;}
    await new Promise(resolve=>setTimeout(resolve,100));
   }
  }catch(error){if(child.exitCode===null)throw error;}
 }
 await exited;}},async open(page){const {bootstrap}=await control("bootstrap");await page.goto(`${origin}/?tool=sample#bootstrap=${bootstrap}`);}};
 try{await host.start();await use(host);}finally{await host.stop();await rm(temporary,{recursive:true,force:true});}
}});
export {expect};
