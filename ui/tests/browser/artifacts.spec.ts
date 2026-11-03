import {test,expect} from "./tool-host";
test.describe("saved operation artifacts",()=>{test.use({hostScenario:"v04-operations"});test("output inspection follows a saved reference after reopening the host",async({page,host})=>{
 await host.open(page);await page.getByRole("button",{name:"Workspace Playground",exact:true}).click();
 await page.getByRole("dialog",{name:"Choose workspace"}).getByRole("button",{name:new RegExp("^Operations ")}).click();
 await page.getByLabel("Import image",{exact:true}).setInputFiles("tests/fixtures/sample.png");
 await expect(page.getByText(/Image ·/)).toBeVisible();await page.getByRole("button",{name:"Run",exact:true}).click();
 const outputs=page.getByRole("region",{name:"Operation outputs"});
 await outputs.getByRole("button",{name:/Inspect marker/}).click();
 await expect(outputs.getByText(/Artifact resource · native record/)).toBeVisible();
 await host.stop();await host.start();await host.open(page);
 await page.getByRole("button",{name:"History",exact:true}).click();
 await page.getByRole("dialog").getByRole("button",{name:/locate_marker/}).first().click();
 await outputs.getByRole("button",{name:/Inspect marker/}).click();
 await expect(outputs.getByText(/Artifact resource · native record/)).toBeVisible();
 });});
test("previewless duplicate output cards keep identity when inspection fails",async({page})=>{
 await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-artifactoutputs--missing-store&viewMode=story");
 await expect(page.getByText("No preview supplied.")).toHaveCount(2);
 await page.getByRole("button",{name:"Inspect masks item 2",exact:true}).click();
 await expect(page.getByRole("alert")).toContainText("Import the saved bundle");
 await expect(page.getByRole("heading",{name:"masks · study:Mask"})).toHaveCount(2);
});
