import {test,expect} from "./tool-host";
test.describe("independent operation chains",()=>{test.use({hostScenario:"v04-operations"});test("send to fills drafts and native scalar/list bindings run without repeating upstream",async({page,host})=>{
 await host.open(page);await page.getByRole("button",{name:"Workspace Playground",exact:true}).click();
 await page.getByRole("dialog",{name:"Choose workspace"}).getByRole("button",{name:new RegExp("^Operations ")}).click();
 await page.getByLabel("Import image",{exact:true}).setInputFiles("tests/fixtures/sample.png");
 await expect(page.getByText(/Image ·/)).toBeVisible();await page.getByRole("button",{name:"Run",exact:true}).click();
 await page.getByRole("button",{name:"Use marker item 1 in another action",exact:true}).click();
 await page.getByRole("dialog",{name:"Send artifact to"}).getByRole("button",{name:/stamp marker.*marker/}).click();
 await expect(page.getByRole("button",{name:/^Operation stamp marker/})).toBeVisible();
 await expect(page.getByRole("region",{name:"Operation outputs"})).toHaveCount(0);
 await page.getByRole("combobox",{name:"Saved image",exact:true}).selectOption({index:1});
 await page.getByRole("button",{name:"Run",exact:true}).click();
 await page.getByRole("button",{name:"Use token item 1 in another action",exact:true}).click();
 await page.getByRole("dialog",{name:"Send artifact to"}).getByRole("button",{name:/soften token.*token/}).click();
 await expect(page.getByRole("button",{name:/^Operation soften token/})).toBeVisible();
 await page.getByRole("button",{name:"Run",exact:true}).click();
 await page.getByRole("button",{name:"Use token item 1 in another action",exact:true}).click();
 await page.getByRole("dialog",{name:"Send artifact to"}).getByRole("button",{name:/collect tokens.*tokens/}).click();
 await expect(page.getByRole("button",{name:/^Operation collect tokens/})).toBeVisible();
 const tokens=page.locator("fieldset").filter({has:page.locator("legend",{hasText:"tokens"})});
 await tokens.getByRole("combobox",{name:"Saved artifact",exact:true}).selectOption({index:1});
 await expect(tokens.getByRole("status",{name:"tokens selection count"})).toHaveText("2 selected");
 await page.getByRole("button",{name:"Run",exact:true}).click();
 await expect(page.getByRole("region",{name:"Operation outputs"}).getByRole("article")).toHaveCount(2);
 await page.getByRole("button",{name:"History",exact:true}).click();
 await expect(page.getByRole("dialog",{name:"History"}).getByRole("button",{name:/sample \/ (locate_marker|stamp_marker|soften_token|collect_tokens)/})).toHaveCount(4);
});});
import {visualPolicy} from "./visual-policy";
for(const width of [1440,390])test(`saved artifact chain list at ${width}`,async({page})=>{
 await page.setViewportSize({width,height:900});
 await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-porteditor--saved-chain&viewMode=story");
 await expect(page.getByRole("status")).toHaveText("2 selected");
 await page.getByRole("button",{name:"Move mask item 2 up",exact:true}).click();
 await page.mouse.move(0,0);
 await expect(page.locator("#storybook-root")).toHaveScreenshot(`artifact-chain-${width}.png`,visualPolicy);
});

test.describe("asynchronous destination selection",()=>{
 test.use({hostScenario:"v04-operations"});
 test("Run stays locked until the checked destination draft replaces the source",async({page,host})=>{
  await host.open(page);await page.getByRole("button",{name:"Workspace Playground",exact:true}).click();
  await page.getByRole("dialog",{name:"Choose workspace"}).getByRole("button",{name:new RegExp("^Operations ")}).click();
  await page.getByLabel("Import image",{exact:true}).setInputFiles("tests/fixtures/sample.png");
  await expect(page.getByText(/Image ·/)).toBeVisible();await page.getByRole("button",{name:"Run",exact:true}).click();
  await page.getByRole("button",{name:"Use marker item 1 in another action",exact:true}).click();
  let release!:()=>void,started!:()=>void,finished!:()=>void;
  const held=new Promise<void>(resolve=>{release=resolve;}),seen=new Promise<void>(resolve=>{started=resolve;}),drained=new Promise<void>(resolve=>{finished=resolve;});
  await page.route(/\/api\/attempts\/[^/]+\/artifacts\/out\/0\?target_tool=sample$/,async route=>{
   try{const response=await route.fetch();started();await held;await route.fulfill({response});}finally{finished();}
  });
  try{
   await page.getByRole("dialog",{name:"Send artifact to"}).getByRole("button",{name:/stamp marker.*marker/}).click();
   await seen;
   await expect(page.getByRole("button",{name:"Run",exact:true})).toBeDisabled();
   await expect(page.getByRole("button",{name:/^Operation locate marker/})).toBeVisible();
  }finally{release();await drained;}
  await expect(page.getByRole("button",{name:/^Operation stamp marker/})).toBeVisible();
  await expect(page.locator("fieldset[data-port=marker]").getByRole("status",{name:"marker selection count"})).toHaveText("1 selected");
  await expect(page.getByRole("region",{name:"Operation outputs"})).toHaveCount(0);
  await page.getByRole("button",{name:"History",exact:true}).click();
  await expect(page.getByRole("dialog",{name:"History"}).getByRole("button",{name:/sample \/ locate_marker/})).toHaveCount(1);
 });
});
