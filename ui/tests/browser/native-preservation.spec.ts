import {rm,readFile} from "node:fs/promises";
import {dirname,join} from "node:path";
import {test,expect} from "./tool-host";
test.describe("native artifact preservation",()=>{
 test.use({hostScenario:"v04-operations"});
 test("missing tool bytes require explicit import and resource verification before binding",async({page,host},testInfo)=>{
  await host.open(page);
  await page.getByRole("button",{name:"Workspace Playground",exact:true}).click();
  await page.getByRole("dialog",{name:"Choose workspace"}).getByRole("button",{name:new RegExp("^Operations ")}).click();
  await page.getByLabel("Import image",{exact:true}).setInputFiles("tests/fixtures/sample.png");
  await expect(page.getByText(/Image ·/)).toBeVisible();
  await page.getByRole("button",{name:"Run",exact:true}).click();
  await expect(page.getByRole("region",{name:"Native preservation"})).toContainText("Native artifacts · preserved");
  const tool=join(dirname(host.root),"tool");
  await rm(join(tool,"artifacts"),{recursive:true});
  await page.getByRole("button",{name:"Use marker item 1 in another action",exact:true}).click();
  await page.getByRole("dialog",{name:"Send artifact to"}).getByRole("button",{name:/stamp marker.*marker/}).click();
  const restore=page.getByRole("dialog",{name:"Restore saved artifact"});
  await expect(restore).toContainText("Verified native bytes");
  await page.screenshot({path:testInfo.outputPath("native-restore.png")});
  await expect(page.getByRole("button",{name:/^Operation stamp marker/})).toHaveCount(0);
  let calls=(await readFile(join(tool,"control-calls.jsonl"),"utf8")).trim().split("\n").map(line=>JSON.parse(line));
  expect(calls.filter(call=>call.name==="import")).toHaveLength(0);
  await restore.getByRole("button",{name:"Import saved copy",exact:true}).click();
  await expect(restore).not.toBeVisible();
  await expect(page.getByRole("button",{name:/^Operation stamp marker/})).toBeVisible();
  await expect(page.locator("fieldset[data-port=marker]").getByRole("status")).toHaveText("1 selected");
  calls=(await readFile(join(tool,"control-calls.jsonl"),"utf8")).trim().split("\n").map(line=>JSON.parse(line));
  expect(calls.filter(call=>call.name==="import")).toHaveLength(1);
  await page.getByRole("button",{name:"History",exact:true}).click();
  await expect(page.getByRole("dialog",{name:"History"}).getByRole("button",{name:/sample \/ locate_marker/})).toHaveCount(1);
 });
});
test("preservation failures expose an explicit retry without changing the run outcome",async({page})=>{
 await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-nativepreservation--failed&viewMode=story");
 await expect(page.getByRole("region",{name:"Native preservation"})).toContainText("original action will not be replayed");
 await page.getByRole("button",{name:"Try again"}).click();
 await expect(page.getByRole("button",{name:"Try again"})).toBeEnabled();
});
