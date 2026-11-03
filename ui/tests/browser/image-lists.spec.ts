import { test, expect } from "./tool-host";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

test.use({ hostScenario: "image-list" });

test("ordered references can be edited, restored from history and omitted", async ({ page, host }, testInfo) => {
  await host.open(page);
  const source = page.getByRole("region", {name:"source", exact:true});
  const refs = page.getByRole("region", {name:"references · reference", exact:true});
  await source.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
  await expect(source.getByRole("img")).toBeVisible();
  await expect(refs.locator("input[type=file]")).toHaveAttribute("multiple", "");
  await refs.locator("input[type=file]").setInputFiles([
    "tests/fixtures/sample.png", "tests/fixtures/panorama.png", "tests/fixtures/sample.png",
  ]);
  await expect(refs).toHaveAttribute("aria-busy", "false");
  await expect(refs.getByRole("listitem")).toHaveCount(3);
  await refs.getByRole("button", {name:"Move image 2 earlier"}).focus();
  await page.keyboard.press("Enter");
  await expect(refs.getByRole("listitem").first()).toContainText("2048 × 1024");
  await refs.getByRole("button", {name:"Remove image 3"}).click();
  await expect(refs.getByRole("listitem")).toHaveCount(2);
  expect((await refs.getByRole("combobox").first().boundingBox())!.height).toBeGreaterThanOrEqual(40);
  for (const width of [320,768,1024,1440]) {
    await page.setViewportSize({width,height:900});
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await refs.getByRole("combobox").first().scrollIntoViewIfNeeded();
    await page.screenshot({path:testInfo.outputPath(`image-list-${width}.png`),fullPage:true});
  }
  const label = page.getByLabel("Label", {exact:true});
  await label.click(); await label.fill("Ordered references");
  await expect(label).toHaveValue("Ordered references");
  await page.getByRole("button", {name:"Run",exact:true}).click();
  await expect(page.getByText("Contract evidence: complete.", {exact:false}).first()).toBeVisible();
  const calls = () => readFile(resolve(host.root,"../tool/calls.ndjson"),"utf8").then(text => text.trim().split("\n").map(line => JSON.parse(line)));
  const first = (await calls())[0].arguments;
  expect(first.references.map((entry: {expect:{width:number}}) => entry.expect.width)).toEqual([2048,160]);
  await refs.getByRole("button", {name:"Remove image 1"}).click();
  await page.getByRole("button", {name:"History",exact:true}).click();
  await page.getByRole("dialog", {name:"History",exact:true}).getByRole("button", {name:/sample \/ mark_band · completed/}).click();
  await expect(refs.getByRole("listitem")).toHaveCount(2);
  await expect(refs.getByRole("listitem").first()).toContainText("2048 × 1024");
  await expect(label).toHaveValue("Ordered references");
  await refs.getByRole("button", {name:"Remove image 1"}).click();
  await refs.getByRole("button", {name:"Remove image 1"}).click();
  await page.getByRole("button", {name:"Run",exact:true}).click();
  await expect.poll(async () => (await calls()).length).toBe(2);
  expect((await calls())[1].arguments.references).toBeUndefined();
  await expect(page.getByText("Contract evidence: complete.", {exact:false}).first()).toBeVisible();
});

test("duplicate image coordinates follow reorder and survive a saved example", async ({ page, host }) => {
  await host.open(page);
  const source = page.getByRole("region", {name:"source",exact:true});
  const refs = page.getByRole("region", {name:"references · reference",exact:true});
  await source.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
  await expect(source.getByRole("img")).toBeVisible();
  await refs.locator("input[type=file]").setInputFiles(["tests/fixtures/panorama.png","tests/fixtures/panorama.png"]);
  await expect(refs).toHaveAttribute("aria-busy","false");
  const frame = (index:number) => refs.getByRole("combobox", {name:`references image ${index} coordinates`,exact:true});
  await frame(2).selectOption("equirectangular");
  await refs.getByRole("button", {name:"Move image 2 earlier"}).click();
  await expect(frame(1)).toHaveValue("equirectangular");
  await expect(frame(2)).toHaveValue("flat");
  const label = page.getByLabel("Label", {exact:true});
  await label.click(); await label.fill("Distinct duplicate frames");
  await expect(label).toHaveValue("Distinct duplicate frames");
  await page.getByRole("button", {name:"Run",exact:true}).click();
  // The independent processor supports flat frames only; its refusal still saves exact inputs.
  await expect(page.getByRole("status").filter({hasText:"invalid"})).toBeVisible();
  await expect(page.getByText("Contract evidence: incomplete.", {exact:false}).first()).toBeVisible();
  await page.getByRole("button", {name:"Examples",exact:true}).click();
  const drawer = page.getByRole("dialog", {name:"Examples",exact:true});
  await drawer.getByLabel("Example name", {exact:true}).fill("Ordered frames");
  await drawer.getByRole("button", {name:"Save example",exact:true}).click();
  await expect(drawer.getByRole("button", {name:"Open Ordered frames"})).toBeVisible();
  await page.keyboard.press("Escape");
  await refs.getByRole("button", {name:"Remove image 1"}).click();
  await expect(frame(1)).toHaveValue("flat");
  await page.getByRole("button", {name:"Examples",exact:true}).click();
  await drawer.getByRole("button", {name:"Open Ordered frames"}).click();
  await expect(refs.getByRole("listitem")).toHaveCount(2);
  await expect(frame(1)).toHaveValue("equirectangular");
  await expect(frame(2)).toHaveValue("flat");
  const calls = (await readFile(resolve(host.root,"../tool/calls.ndjson"),"utf8")).trim().split("\n");
  expect(calls).toHaveLength(1);
});

test("an import preserves saved-image insertions and cannot overwrite a restored draft", async ({page, host}) => {
  await host.open(page);
  const source = page.getByRole("region", {name:"source",exact:true});
  const refs = page.getByRole("region", {name:"references · reference",exact:true});
  await source.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
  await expect(source.getByRole("img")).toBeVisible();
  await refs.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
  await expect(refs.getByRole("img")).toBeVisible();
  const label = page.getByLabel("Label", {exact:true});
  await label.click(); await label.fill("Saved before import");
  await expect(label).toHaveValue("Saved before import");
  await page.getByRole("button", {name:"Run",exact:true}).click();
  await expect(page.getByText("Contract evidence: complete.",{exact:false}).first()).toBeVisible();
  await page.getByRole("button",{name:"Examples",exact:true}).click();
  const drawer = page.getByRole("dialog",{name:"Examples",exact:true});
  await drawer.getByLabel("Example name",{exact:true}).fill("Before import");
  await drawer.getByRole("button",{name:"Save example",exact:true}).click();
  await expect(drawer.getByRole("button",{name:"Open Before import"})).toBeVisible();
  await page.keyboard.press("Escape");
  for (const mode of ["restore", "append"]) {
    let release = () => {};
    let captured = () => {};
    const gate = new Promise<void>(resolve => {release = resolve;});
    const received = new Promise<void>(resolve => {captured = resolve;});
    await page.route("**/api/assets", async route => {
      if (route.request().method() !== "POST") { await route.continue(); return; }
      // Use explicit fixture bytes for the held upload in both engines.
      const response = await route.fetch({postData:await readFile("tests/fixtures/panorama.png")});
      expect(response.status()).toBe(201);
      captured(); await gate;
      await route.fulfill({response});
    });
    try {
      await refs.getByRole("combobox").first().selectOption("flat-wrap");
      await refs.locator("input[type=file]").setInputFiles("tests/fixtures/panorama.png");
      await received;
      if (mode === "append") {
        await page.locator("summary").filter({hasText:"Saved images"}).click();
        await page.getByText("Use for references",{exact:true}).locator("..").getByRole("button").first().click();
        await expect(refs.getByRole("listitem")).toHaveCount(2);
      } else {
        await page.getByRole("button",{name:"Examples",exact:true}).click();
        await drawer.getByRole("button",{name:"Open Before import"}).click();
        await expect(refs.getByRole("listitem")).toHaveCount(1);
      }
      release();
      await page.unrouteAll({behavior:"wait"});
      await expect(page.locator("summary").filter({hasText:"Saved images"})).toContainText("(3)");
      if (mode === "append") {
        await expect(refs).toHaveAttribute("aria-busy","false");
        await expect(refs.getByRole("listitem")).toHaveCount(3);
        await expect(refs.getByRole("combobox").first()).toHaveValue("flat-wrap");
      } else {
        // The restored editor has a new owner; wait for the old request's final asset-list read.
        await expect(refs.getByRole("listitem")).toHaveCount(1);
        await expect(label).toHaveValue("Saved before import");
        await expect(refs.getByRole("combobox").first()).toHaveValue("flat");
      }
    } finally { release(); await page.unrouteAll({behavior:"wait"}); }
  }
});

test("input thumbnails never request the full original just to display a card",async({page,host})=>{
 const originalReads:string[]=[];
 page.on("request",request=>{if(request.method()==="GET"&&/\/api\/assets\/[a-f0-9]{32}$/.test(new URL(request.url()).pathname))originalReads.push(request.url());});
 await host.open(page);
 const thumbnail=page.waitForResponse(response=>/\/api\/assets\/[a-f0-9]{32}\/thumbnail$/.test(new URL(response.url()).pathname));
 const source=page.getByRole("region",{name:"source",exact:true});
 await source.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");
 expect((await thumbnail).status()).toBe(200);
 const image=source.getByRole("img");await expect(image).toBeVisible();
 await expect.poll(()=>image.evaluate((node:HTMLImageElement)=>node.naturalWidth)).toBeGreaterThan(0);
 expect(await image.evaluate((node:HTMLImageElement)=>Math.max(node.naturalWidth,node.naturalHeight))).toBeLessThanOrEqual(512);
 expect(originalReads).toEqual([]);
});
