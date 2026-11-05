import {test,expect} from "./tool-host";
test("typed ports keep list occurrences and show indexed refusal at narrow width",async({page})=>{
 await page.setViewportSize({width:390,height:900});
 await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-porteditor--refused&viewMode=story");
 const mask=page.locator("fieldset").filter({has:page.locator("legend",{hasText:"masks"})});
 await expect(mask.getByRole("alert")).toContainText("Import the saved bundle.");
 await mask.getByRole("button",{name:"Move masks item 2 up",exact:true}).click();
 await mask.getByRole("button",{name:"Remove masks item 2",exact:true}).click();
 await expect(mask.locator("li")).toHaveCount(1);
 expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
});
test.describe("operation images",()=>{test.use({hostScenario:"v04-operations"});test("an image port runs through the workspace",async({page,host})=>{
 await host.open(page);await page.getByRole("button",{name:"Workspace Playground",exact:true}).click();
 await page.getByRole("dialog",{name:"Choose workspace"}).getByRole("button",{name:new RegExp("^Operations ")}).click();
 await page.getByLabel(/^Use your own image as /).first().setInputFiles("tests/fixtures/sample.png");
 await expect(page.getByRole("status",{name:"image selection count"})).toHaveText("1 selected");await page.getByRole("button",{name:"Run",exact:true}).click();
 await expect(page.getByText("Saved attempt",{exact:false})).toBeVisible();
 });});
test("a deferred import preserves edits to another port",async({page})=>{
 await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-porteditor--deferred-import&viewMode=story");
 await page.getByLabel(/^Use your own image as /).first().setInputFiles("tests/fixtures/sample.png");
 await expect(page.getByRole("status").filter({hasText:"Importing"})).toBeVisible();
 const masks=page.locator("fieldset").filter({has:page.locator("legend",{hasText:"masks"})});
 await masks.getByLabel("Saved artifact").selectOption("mask");
 await page.getByRole("button",{name:"Finish import"}).click();
 await expect(page.getByTestId("bindings")).toContainText('"masks"');
 await expect(page.getByTestId("bindings")).toContainText('"image"');
});
