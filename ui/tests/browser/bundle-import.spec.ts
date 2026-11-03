import {test,expect} from "./tool-host";
import {prepareSampleInput} from "./sample-input";
test("an uploaded bundle is reviewed before explicit input import and never starts a run",async({page,host},info)=>{
 await host.open(page);await prepareSampleInput(page,"Portable input");await page.getByRole("button",{name:"Run",exact:true}).click();
 await expect(page.getByText("Contract evidence: complete.",{exact:false}).first()).toBeVisible();
 await page.getByRole("button",{name:"Use in your pipeline",exact:true}).click();
 let drawer=page.getByRole("dialog",{name:"Use in your pipeline",exact:true});
 await drawer.getByRole("button",{name:"Invocation bundle",exact:true}).click();await drawer.getByRole("button",{name:"Save bundle for review",exact:true}).click();
 const downloading=page.waitForEvent("download");await drawer.getByRole("button",{name:"Download saved invocation bundle",exact:true}).click();const download=await downloading;const archive=info.outputPath("import.tar");await download.saveAs(archive);
 await drawer.getByRole("button",{name:"Close",exact:true}).click();await page.getByRole("button",{name:"Bundles",exact:true}).click();
 drawer=page.getByRole("dialog",{name:"Saved invocation bundles",exact:true});
 await drawer.getByText("Import an invocation bundle",{exact:true}).click();
 await drawer.getByLabel("Upload invocation bundle",{exact:true}).setInputFiles(archive);
 const review=drawer.getByRole("region",{name:"Uploaded bundle review",exact:true});
 await expect(review).toContainText("Declared files verified");await expect(review).toContainText("Portable input");
 await expect(review).toContainText("No tool action has been run");await expect(review.getByRole("button",{name:"Run",exact:true})).toHaveCount(0);
 await review.getByRole("button",{name:"Import selected step inputs",exact:true}).click();
 await expect(review.getByRole("status")).toContainText("Inputs imported");await expect(review).toContainText("completed");
 await drawer.getByRole("button",{name:"Close",exact:true}).click();
 await page.getByRole("button",{name:"Bundles",exact:true}).click();await drawer.getByText("Import an invocation bundle",{exact:true}).click();
 await drawer.getByRole("button",{name:/Review uploaded/}).click();await expect(drawer.getByRole("region",{name:"Uploaded bundle review",exact:true}).getByRole("status")).toContainText("Inputs imported");
 await drawer.getByRole("button",{name:"Close",exact:true}).click();await page.getByRole("button",{name:"History",exact:true}).click();
 await expect(page.getByRole("dialog",{name:"History",exact:true}).getByRole("button",{name:/sample \/ mark_band/})).toHaveCount(1);
});
test("an invalid upload reports its refusal without creating a review",async({page,host})=>{
 await host.open(page);await page.getByRole("button",{name:"Bundles",exact:true}).click();const drawer=page.getByRole("dialog",{name:"Saved invocation bundles",exact:true});
 await drawer.getByText("Import an invocation bundle",{exact:true}).click();
 await drawer.getByLabel("Upload invocation bundle",{exact:true}).setInputFiles({name:"bad.tar",mimeType:"application/x-tar",buffer:Buffer.from("not an archive")});
 await expect(drawer.getByRole("region",{name:"Bundle uploads",exact:true})).toContainText("bounded, uncompressed ustar");
 await expect(drawer.getByRole("region",{name:"Uploaded bundle review",exact:true})).toHaveCount(0);
});
import {visualPolicy} from "./visual-policy";
for(const width of [1440,390])for(const state of ["review","partial"])test(`uploaded ${state} review at ${width}`,async({page})=>{
 await page.setViewportSize({width,height:900});await page.goto(`http://127.0.0.1:6006/iframe.html?id=workspace-bundleimports--${state}&viewMode=story`);
 await page.getByRole("button",{name:/Review uploaded/}).click();const drawer=page.getByRole("dialog",{name:"Import invocation bundle",exact:true});
 await expect(drawer.getByRole("region",{name:"Uploaded bundle review",exact:true})).toContainText("Declared files verified");
 await expect(drawer).toContainText(state==="partial"?"Completed inputs are retained":"Tool is not installed");
 expect(await drawer.evaluate(node=>node.scrollWidth<=node.clientWidth)).toBe(true);
 const clipped=await drawer.locator("p,label,select,input").evaluateAll(nodes=>nodes.filter(node=>{const box=node.getBoundingClientRect();return box.width>0&&(box.left<0||box.right>window.innerWidth);}).map(node=>node.tagName));
 expect(clipped).toEqual([]);
 await page.getByRole("heading",{name:"Import invocation bundle",exact:true}).click();await page.mouse.move(0,0);
 await expect(drawer).toHaveScreenshot(`bundle-upload-${state}-${width}.png`,visualPolicy);
 if(state==="partial")await expect(drawer.getByRole("region",{name:"Input import progress",exact:true})).toHaveScreenshot(`bundle-import-progress-${width}.png`,visualPolicy);
});

test("loading, refused and active import reviews expose no Run action",async({page})=>{
 await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-bundleimports--loading&viewMode=story");await expect(page.getByRole("status")).toHaveText("Reading uploaded reviews…");await expect(page.getByLabel("Upload invocation bundle",{exact:true})).toBeDisabled();
 await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-bundleimports--failed&viewMode=story");await expect(page.getByRole("alert")).toContainText("fingerprint failed");
 await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-bundleimports--importing&viewMode=story");await page.getByRole("button",{name:/Review uploaded/}).click();
 await expect(page.getByRole("region",{name:"Input import progress",exact:true}).getByRole("status")).toContainText("Importing inputs");await expect(page.getByRole("button",{name:"Import selected step inputs",exact:true})).toBeDisabled();await expect(page.getByRole("button",{name:"Run",exact:true})).toHaveCount(0);
});
