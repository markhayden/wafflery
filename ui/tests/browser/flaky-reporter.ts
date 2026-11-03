import {appendFileSync} from "node:fs";
import type {FullConfig,Reporter,Suite} from "@playwright/test/reporter";
const commandText=(value:string)=>value.replaceAll("%","%25").replaceAll("\r","%0D").replaceAll("\n","%0A");
const markdownText=(value:string)=>value.replaceAll("&","&amp;").replaceAll("<","&lt;").replaceAll(">","&gt;").replace(/[\\`*_{}\[\]()#!|]/g,"\\$&").replace(/[\r\n]+/g," ");
export function flakyReport(tests:{title:string;retries:number}[]){
 return {
  markdown:`### Browser retry report\n\n${tests.length?`${tests.length} browser test${tests.length===1?"":"s"} passed on retry. Treat each as a bug to investigate.\n\n${tests.map(t=>`- ${markdownText(t.title)} (${t.retries} retry)`).join("\n")}`:"No browser tests passed on retry."}\n`,
  annotations:tests.map(t=>`::warning title=Flaky browser test::${commandText(t.title)} passed on retry; investigate the retained browser diagnostics.`),
 };
}
/** Report flakiness without changing Playwright's final status or hiding failed attempts. */
export default class FlakyReporter implements Reporter {
 private suite:Suite|undefined;
 onBegin(_config:FullConfig,suite:Suite){this.suite=suite;}
 onEnd(){
  const tests=this.suite?.allTests().filter(test=>test.outcome()==="flaky").map(test=>({title:test.titlePath().filter(Boolean).join(" › "),retries:test.results.length-1}))??[];
  const report=flakyReport(tests);
  console.log(report.markdown.trim());for(const annotation of report.annotations)console.log(annotation);
  if(process.env.GITHUB_STEP_SUMMARY)appendFileSync(process.env.GITHUB_STEP_SUMMARY,report.markdown);
  if(process.env.GITHUB_OUTPUT)appendFileSync(process.env.GITHUB_OUTPUT,`flaky_count=${tests.length}\n`);
 }
}
