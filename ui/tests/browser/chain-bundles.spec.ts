import {test,expect} from "./tool-host";
import {prepareSampleInput,editSampleLabel} from "./sample-input";
test("explicit ordered selection exports only selected runs and never dispatches",async({page,host})=>{
 await host.open(page);await prepareSampleInput(page,"First chain step");await page.getByRole("button",{name:"Run",exact:true}).click();await expect(page.getByText("Contract evidence: complete.",{exact:false}).first()).toBeVisible();
 await editSampleLabel(page,"Second chain step");await page.getByRole("button",{name:"Run",exact:true}).click();await expect(page.getByRole("region",{name:"Recent attempts",exact:true}).getByRole("button")).toHaveCount(2);
 await page.getByRole("button",{name:"Bundles",exact:true}).click();const drawer=page.getByRole("dialog",{name:"Saved invocation bundles",exact:true});await drawer.getByText("Build a bundle from saved runs",{exact:true}).click();
 const builder=drawer.getByRole("region",{name:"Chain selection",exact:true});await expect(builder.getByRole("checkbox")).toHaveCount(2);await builder.getByRole("checkbox").nth(1).check();await builder.getByRole("checkbox").nth(0).check();
 await builder.getByRole("button",{name:"Move step 2 up",exact:true}).click();await expect(builder.getByRole("list",{name:"Selected step order"}).getByRole("listitem")).toHaveCount(2);
 await builder.getByRole("button",{name:"Save selected chain for review",exact:true}).click();const review=drawer.getByRole("region",{name:"Saved bundle review",exact:true});await expect(review.getByLabel("Bundle step 2",{exact:true})).toBeVisible();
 await review.getByLabel("Bundle step 1",{exact:true}).getByText("Recorded invocation",{exact:true}).click();await expect(review.getByLabel("Bundle step 1",{exact:true})).toContainText("Second chain step");await expect(review.getByRole("region",{name:"Chain dependencies",exact:true})).toContainText("external inputs");
 await drawer.getByRole("button",{name:"Close",exact:true}).click();await expect(page.getByRole("region",{name:"Recent attempts",exact:true}).getByRole("button")).toHaveCount(2);
});
import {visualPolicy} from "./visual-policy";
for(const width of [390,1440])for(const state of ["chain","mixed-tools","ambiguous-chain"])test(`chain review ${state} at ${width}`,async({page})=>{
 await page.setViewportSize({width,height:900});await page.goto(`http://127.0.0.1:6006/iframe.html?id=workspace-invocationbundles--${state}&viewMode=story`);
 const graph=page.getByRole("region",{name:"Chain dependencies",exact:true});await expect(graph).toBeVisible();if(state==="ambiguous-chain"){await expect(graph).toContainText("single-step bundle");await expect(page.getByLabel("Bundle CLI example")).toHaveCount(0);}else{await expect(graph).toContainText("Step 1 · mask item 1 → Step 2");if(state==="mixed-tools")await expect(page.getByLabel("Bundle CLI example")).toHaveCount(0);}
 expect(await graph.evaluate(node=>node.scrollWidth<=node.clientWidth)).toBe(true);await page.mouse.move(0,0);await expect(graph).toHaveScreenshot(`chain-${state}-${width}.png`,visualPolicy);
});
for(const width of [390,1440])test(`ordered chain selection at ${width}`,async({page})=>{
 await page.setViewportSize({width,height:900});await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-invocationbundles--selection&viewMode=story");await page.getByText("Build a bundle from saved runs",{exact:true}).click();
 const selection=page.getByRole("region",{name:"Chain selection",exact:true});await selection.getByRole("checkbox").nth(1).check();await selection.getByRole("checkbox").nth(0).check();await expect(selection.getByRole("list",{name:"Selected step order"}).getByRole("listitem")).toHaveCount(2);expect(await selection.evaluate(node=>node.scrollWidth<=node.clientWidth)).toBe(true);await page.mouse.move(0,0);await expect(selection).toHaveScreenshot(`chain-selection-${width}.png`,visualPolicy);
});
test("collapsing selection during an owned save releases the drawer when it completes",async({page,host})=>{
 await host.open(page);await prepareSampleInput(page,"Collapsible selection");await page.getByRole("button",{name:"Run",exact:true}).click();await expect(page.getByText("Contract evidence: complete.",{exact:false}).first()).toBeVisible();
 await page.getByRole("button",{name:"Bundles",exact:true}).click();const drawer=page.getByRole("dialog",{name:"Saved invocation bundles",exact:true});await drawer.getByText("Build a bundle from saved runs",{exact:true}).click();await drawer.getByRole("region",{name:"Chain selection",exact:true}).getByRole("checkbox").check();
 let release!:()=>void,seen!:()=>void,finished!:()=>void;const hold=new Promise<void>(r=>{release=r;}),started=new Promise<void>(r=>{seen=r;}),drained=new Promise<void>(r=>{finished=r;});
 await page.route(/\/api\/bundles$/,async route=>{if(route.request().method()!=="POST"){await route.continue();return;}try{const response=await route.fetch();seen();await hold;await route.fulfill({response});}finally{finished();}});
 try{await drawer.getByRole("button",{name:"Save selected chain for review",exact:true}).click();await started;await drawer.getByText("Build a bundle from saved runs",{exact:true}).click();}finally{release();await drained;}
 await expect(drawer.getByRole("region",{name:"Saved bundle review",exact:true})).toBeVisible();await expect(drawer.getByRole("button",{name:"Download saved invocation bundle",exact:true})).toBeEnabled();
});
