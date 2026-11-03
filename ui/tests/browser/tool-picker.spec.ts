import {test,expect} from "./tool-host";
import {readFile} from "node:fs/promises";
import {visualPolicy} from "./visual-policy";
const rascalDescription="Fully automated mask generation for precise image edits. Describe what to change and rascal finds the region, builds a high-quality mask and protects everything else, across a wide range of use cases.";
const squintDescription="Image evaluation you can trust. Checks an image against the brief and references it was made from, and shows the evidence behind every verdict.";
for(const width of [1440,320])test(`tool rows show logos, versions and searchable descriptions at ${width}px`,async({page})=>{
 await page.setViewportSize({width,height:950});
 await page.goto("http://127.0.0.1:6006/iframe.html?id=components-picker--tools&viewMode=story");
 await page.getByRole("button",{name:"Tool rascal",exact:true}).click();
 const dialog=page.getByRole("dialog",{name:"Choose tool"});
 for(const [name,version,description] of [["rascal","Version 0.1.1",rascalDescription],["squint","Version 0.1.0 (6b9bfa327)",squintDescription]]){
  const row=dialog.getByRole("button",{name:new RegExp(`^${name} `)});
  await expect(row.getByText(description!,{exact:true})).toBeVisible();
  const icon=row.locator("img");await expect(icon).toHaveAttribute("alt","");
  await expect(icon).toHaveAttribute("width","40");await expect(icon).toHaveAttribute("height","40");
  await expect.poll(()=>icon.evaluate((image:HTMLImageElement)=>image.naturalWidth)).toBe(96);
  const titleBox=(await row.locator("strong").boundingBox())!;
  const versionBox=(await row.getByText(version!,{exact:true}).boundingBox())!;
  expect(titleBox.x+titleBox.width).toBeLessThanOrEqual(versionBox.x);
  const descBox=(await row.getByText(description!,{exact:true}).boundingBox())!;
  const dimensions=await row.getByText(description!,{exact:true}).evaluate((node:HTMLElement)=>({height:node.clientHeight,scrollHeight:node.scrollHeight,width:node.clientWidth,scrollWidth:node.scrollWidth,clamp:getComputedStyle(node).getPropertyValue("-webkit-line-clamp")}));
  expect(dimensions.scrollHeight).toBeLessThanOrEqual(dimensions.height+1);expect(dimensions.scrollWidth).toBeLessThanOrEqual(dimensions.width+1);expect(dimensions.clamp).not.toMatch(/^[1-9]/);

  expect((await row.locator("img").boundingBox())!.y).toBeLessThanOrEqual(titleBox.y+2);
  expect(descBox.y).toBeGreaterThanOrEqual(Math.max(titleBox.y+titleBox.height,versionBox.y+versionBox.height));
 }
 const unknown=dialog.getByRole("button",{name:/^another-tool /});
 expect((await unknown.locator("strong").boundingBox())!.x).toBe((await dialog.getByRole("button",{name:/^rascal /}).locator("strong").boundingBox())!.x);
 await expect(unknown).toContainText("Explores a locally installed tool using its declared purpose.");await expect(unknown.locator("img")).toHaveCount(0);
 const failed=dialog.getByRole("button",{name:/^offline-tool /});
 await expect(failed).toContainText("Cannot start this tool. Check its installation.");await expect(failed).not.toContainText("This purpose loses");
 expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
 await page.mouse.move(0,0);await expect(dialog).toHaveScreenshot(`tool-picker-${width}.png`,visualPolicy);
 const search=dialog.getByRole("textbox",{name:"Search tool"});
 await search.fill("protects everything else");await expect(dialog.locator("button[aria-pressed]")).toHaveCount(1);await expect(dialog.locator("strong")).toHaveText("rascal");
 await search.fill("evidence behind every verdict");await expect(dialog.locator("strong")).toHaveText("squint");
 await search.fill("6b9bfa327");await expect(dialog.locator("strong")).toHaveText("squint");
 await search.fill("declared purpose");await expect(dialog.locator("strong")).toHaveText("another-tool");
 await search.fill("rascal");await dialog.getByRole("button",{name:/^rascal /}).focus();await expect(dialog.getByRole("button",{name:/^rascal /})).toBeFocused();await page.keyboard.press("Enter");
 await expect(dialog).not.toBeVisible();await expect(page.getByRole("button",{name:"Tool rascal",exact:true})).toBeFocused();
});
for(const width of [1440,320])test(`long tool names and versions fit at ${width}px`,async({page})=>{
 await page.setViewportSize({width,height:950});await page.goto("http://127.0.0.1:6006/iframe.html?id=components-picker--long-tool-version&viewMode=story");
 await page.getByRole("button",{name:"Tool rascal",exact:true}).click();const dialog=page.getByRole("dialog",{name:"Choose tool"});
 for(const row of await dialog.locator("button[aria-pressed]").all()){
  const title=(await row.locator("strong").boundingBox())!;const version=(await row.locator("small[title]").boundingBox())!;
  expect(title.x+title.width).toBeLessThanOrEqual(version.x);expect(version.x+version.width).toBeLessThanOrEqual((await row.boundingBox())!.x+(await row.boundingBox())!.width);
 }
 await expect(dialog).toHaveScreenshot(`tool-picker-long-${width}.png`,visualPolicy);
 expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
});

test("the packaged host serves unchanged WebP logos under its existing CSP",async({page,host})=>{
 const errors:string[]=[];page.on("pageerror",error=>errors.push(error.message));page.on("console",message=>{if(message.type()==="error")errors.push(message.text());});
 await host.open(page);
 const pageResponse=await page.request.get(host.origin+"/");expect(pageResponse.headers()["content-security-policy"]).toContain("img-src 'self' blob:");
 const purpose=await page.getByRole("heading",{name:"sample-lab",exact:true}).locator("..").locator("p").innerText();
 await page.getByRole("button",{name:"Tool sample-lab",exact:true}).click();
 const unknown=page.getByRole("dialog",{name:"Choose tool"}).getByRole("button",{name:/^sample-lab /});
 await expect(unknown.getByText("Version 0.1.0",{exact:true})).toBeVisible();
 await expect(unknown.getByText(purpose,{exact:true})).toBeVisible();await expect(unknown.locator("img")).toHaveCount(0);
 const manifest=JSON.parse(await readFile(new URL("../../prebuilt/manifest.json",import.meta.url),"utf8"));
 for(const name of ["rascal","squint"]){
  const asset=Object.keys(manifest.outputs).find(path=>path.startsWith(`assets/${name}-`)&&path.endsWith(".webp"));expect(asset).toBeDefined();
  const response=await page.request.get(`${host.origin}/${asset}`);expect(response.status()).toBe(200);expect(response.headers()["content-type"]).toBe("image/webp");
  expect(response.headers()["content-security-policy"]).toContain("img-src 'self' blob:");
  expect(await response.body()).toEqual(await readFile(new URL(`../../src/assets/tools/${name}.webp`,import.meta.url)));
  // Exercise image decoding under the real page's CSP without inventing MCP registrations.
  await unknown.locator("span[aria-hidden=true]").evaluate((mark,src)=>{
   const image=document.createElement("img");image.src=src;image.alt="";image.width=40;image.height=40;
   mark.replaceChildren(image);
  },host.origin+"/"+asset);
  await expect.poll(()=>unknown.locator("img").evaluate((image:HTMLImageElement)=>image.complete&&image.naturalWidth===96)).toBe(true);
 }
 expect(errors).toEqual([]);
});
