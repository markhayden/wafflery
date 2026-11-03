import {test,expect} from "bun:test";
import {flakyReport} from "../browser/flaky-reporter";
test("CI summary lists flaky tests and safely formats command and markdown text",()=>{
 const report=flakyReport([{title:"[webkit] estimates › restart",retries:1},{title:"bad\n::notice::title <script> %",retries:1}]);
 expect(report.markdown).toContain("2 browser tests passed on retry");expect(report.markdown).toContain("\\[webkit\\] estimates › restart");expect(report.markdown).not.toContain("<script>");expect(report.annotations).toHaveLength(2);expect(report.annotations[1]).toContain("%0A::notice::title <script> %25");expect(report.annotations[1]).not.toContain("\n");
 expect(flakyReport([]).markdown).toContain("No browser tests passed on retry");
});
import {mkdtemp,readFile,writeFile,rm} from "node:fs/promises";
import {resolve} from "node:path";
test("the real reporter distinguishes a recovered retry from a twice-failing test",async()=>{
 const ui=resolve(import.meta.dir,"../..");const dir=await mkdtemp(resolve(ui,".retry-reporter-"));
 try{
  const reporter=resolve(ui,"tests/browser/flaky-reporter.ts");
  await writeFile(resolve(dir,"config.ts"),`import {defineConfig} from '@playwright/test';export default defineConfig({testDir:${JSON.stringify(dir)},outputDir:${JSON.stringify(resolve(dir,"results"))},workers:1,retries:1,reporter:[['dot'],[${JSON.stringify(reporter)}]]});`);
  for(const permanent of [false,true]){
   const summary=resolve(dir,`summary-${permanent}.md`),output=resolve(dir,`output-${permanent}.txt`);
   await writeFile(resolve(dir,"retry.spec.ts"),`import {test,expect} from '@playwright/test';test('synthetic retry result',async({},info)=>{expect(${permanent?"false":"info.retry>0"}).toBe(true);});`);
   const child=Bun.spawn({cmd:[process.execPath,"x","playwright","test","--config",resolve(dir,"config.ts")],cwd:ui,env:{...process.env,CI:"true",GITHUB_STEP_SUMMARY:summary,GITHUB_OUTPUT:output},stdout:"pipe",stderr:"pipe"});
   const [code,stdout,stderr]=await Promise.all([child.exited,new Response(child.stdout).text(),new Response(child.stderr).text()]);
   expect(code,stdout+stderr).toBe(permanent?1:0);const markdown=await readFile(summary,"utf8");expect(markdown).toContain(permanent?"No browser tests passed on retry":"1 browser test passed on retry");expect(await readFile(output,"utf8")).toBe(`flaky_count=${permanent?0:1}\n`);
   if(!permanent)expect(stdout).toContain("::warning title=Flaky browser test::");
  }
 }finally{await rm(dir,{recursive:true,force:true});}
},30_000);
