import {visualPolicy} from "./visual-policy";
import {test,expect} from "./tool-host";
import {prepareSampleInput} from "./sample-input";
test("CLI handoff reads the selected saved invocation after editing the draft and explains extra tool setup",async({page,host})=>{
 await host.open(page);await prepareSampleInput(page,"Saved CLI input");
 await page.getByRole("button",{name:"Run",exact:true}).click();
 await expect(page.getByText("Contract evidence: complete.",{exact:false}).first()).toBeVisible();
 await page.getByLabel("Label",{exact:true}).fill("Not in the handoff");
 await page.getByRole("button",{name:"Use in your pipeline",exact:true}).click();
 const drawer=page.getByRole("dialog",{name:"Use in your pipeline",exact:true});
 await drawer.getByRole("button",{name:"Command",exact:true}).click();
 const handoff=drawer.getByRole("region",{name:"CLI handoff"});
 await expect(handoff).toContainText("extra launch arguments");
 await handoff.getByText("Resolved settings",{exact:true}).click();
 await expect(handoff.getByLabel("Handoff invocation")).toContainText("Saved CLI input");
 await expect(handoff.getByLabel("Handoff invocation")).not.toContainText("Not in the handoff");
 await expect(handoff.getByRole("button",{name:"Copy command",exact:true})).toHaveCount(0);
});
for(const width of [390,1440])test(`local command setup is readable and does not expose credential values at ${width}`,async({page})=>{
 await page.setViewportSize({width,height:900});
 await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-commandhandoff--local&viewMode=story");
 await expect(page.getByLabel("CLI command")).toContainText("--arguments");
 await expect(page.getByRole("region",{name:"CLI handoff"})).toContainText("PROVIDER_KEY");
 await expect(page.getByRole("button",{name:"Download arguments",exact:true})).toBeEnabled();
 expect(await page.getByRole("dialog").evaluate(node=>node.scrollWidth<=node.clientWidth)).toBe(true);
 await page.getByRole("heading",{name:"Use in your pipeline",exact:true}).click();await page.mouse.move(0,0);
 await expect(page.getByRole("dialog")).toHaveScreenshot(`command-handoff-${width}.png`,visualPolicy);
});

test("Command refreshes when its saved run completes without closing the drawer",async({page})=>{
 await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-commandhandoff--completing&viewMode=story");
 await expect(page.getByRole("region",{name:"CLI handoff"})).toContainText("no current resolved invocation");
 await page.getByRole("button",{name:"Finish saved run",exact:true}).click();
 await expect(page.getByLabel("CLI command")).toBeVisible();
});
test("the local arguments download completes with the saved values",async({page},testInfo)=>{
 await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-commandhandoff--local&viewMode=story");
 const download=page.waitForEvent("download");await page.getByRole("button",{name:"Download arguments",exact:true}).click();
 const file=await download;const path=testInfo.outputPath("wafflery-arguments.json");await file.saveAs(path);
 const {readFile}=await import("node:fs/promises");const args=JSON.parse(await readFile(path,"utf8"));
 expect(args.sigma_px).toBe(6);expect(args.max_spend_usd).toBe(0.3);expect(args.invocation).toBeUndefined();
});
