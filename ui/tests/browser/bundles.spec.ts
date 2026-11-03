import {test,expect} from "./tool-host";
import {prepareSampleInput} from "./sample-input";
import {visualPolicy} from "./visual-policy";
test("a bundle reviews the selected saved inputs and still downloads after source cleanup",async({page,host},testInfo)=>{
 await host.open(page);await prepareSampleInput(page,"Frozen bundle label");await page.getByRole("button",{name:"Run",exact:true}).click();
 await expect(page.getByText("Contract evidence: complete.",{exact:false}).first()).toBeVisible();
 await page.getByLabel("Label",{exact:true}).fill("Draft changed later");
 await page.getByRole("button",{name:"Use in your pipeline",exact:true}).click();
 const drawer=page.getByRole("dialog",{name:"Use in your pipeline",exact:true});
 await drawer.getByRole("button",{name:"Invocation bundle",exact:true}).click();
 await drawer.getByRole("button",{name:"Save bundle for review",exact:true}).click();
 const review=drawer.getByRole("region",{name:"Saved bundle review",exact:true});await expect(review).toContainText("Inputs included");
 await review.getByText("Recorded invocation",{exact:true}).click();await expect(review.locator("pre").first()).toContainText("Frozen bundle label");await expect(review).not.toContainText("Draft changed later");
 const downloading=page.waitForEvent("download");await review.getByRole("button",{name:"Download saved invocation bundle",exact:true}).click();const download=await downloading;const path=testInfo.outputPath("bundle.tar");await download.saveAs(path);
 const {readFile}=await import("node:fs/promises");const original=await readFile(path);expect(original.subarray(0,15).toString()).toContain("invocation.json");
 await drawer.getByRole("button",{name:"Close",exact:true}).click();
 await page.getByRole("button",{name:"Clean up library",exact:true}).click();const cleanup=page.getByRole("dialog",{name:"Clean up library",exact:true});
 await expect(cleanup).toContainText("1 saved invocation bundles retain");await cleanup.getByLabel(/Delete run/).first().check();await cleanup.getByRole("button",{name:"Preview cleanup",exact:true}).click();await cleanup.getByRole("button",{name:"Confirm cleanup",exact:true}).click();await expect(cleanup.getByRole("status")).toContainText("Cleanup complete");await cleanup.getByRole("button",{name:"Close",exact:true}).click();
 await page.getByRole("button",{name:"Bundles",exact:true}).click();const library=page.getByRole("dialog",{name:"Saved invocation bundles",exact:true});await library.getByRole("button",{name:/Review sample-lab/}).click();
 const second=page.waitForEvent("download");await library.getByRole("button",{name:"Download saved invocation bundle",exact:true}).click();const restored=await second;const restoredPath=testInfo.outputPath("restored.tar");await restored.saveAs(restoredPath);expect(await readFile(restoredPath)).toEqual(original);
});
for(const width of [390,1440])for(const state of ["ready","partial"] as const)test(`bundle ${state} review at ${width}`,async({page})=>{
 await page.setViewportSize({width,height:900});await page.goto(`http://127.0.0.1:6006/iframe.html?id=workspace-invocationbundles--${state}&viewMode=story`);
 await page.getByRole("button",{name:/Review study/}).click();const drawer=page.getByRole("dialog");const review=drawer.getByRole("region",{name:"Saved bundle review",exact:true});
 await expect(review).toBeVisible();if(state==="partial"){await expect(review).toContainText("cannot be replayed as-is");await expect(review.getByLabel("Bundle CLI example")).toHaveCount(0);}else await expect(review.getByLabel("Bundle CLI example")).toContainText("call --bundle");
 expect(await drawer.evaluate(node=>node.scrollWidth<=node.clientWidth)).toBe(true);await page.getByRole("heading",{name:"Saved invocation bundles",exact:true}).click();await page.mouse.move(0,0);await expect(drawer).toHaveScreenshot(`bundle-${state}-${width}.png`,visualPolicy);
});
test("loading and failed bundle reads never expose a replay command",async({page})=>{
 for(const [state,message] of [["loading","Reading and verifying"],["failed","fingerprint failed verification"],["empty","No invocation bundles"]]){
  await page.goto(`http://127.0.0.1:6006/iframe.html?id=workspace-invocationbundles--${state}&viewMode=story`);await expect(page.getByRole("region",{name:"Invocation bundles",exact:true})).toContainText(message!);await expect(page.getByLabel("Bundle CLI example")).toHaveCount(0);
 }
});
