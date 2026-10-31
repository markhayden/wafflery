import {test,expect} from "./tool-host";
test.describe("active shutdown",()=>{
  test.use({hostScenario:"slow"});
  test("declining leaves work running; confirming stops the owned host",async({page,host})=>{
    await host.open(page);await page.locator("input[type=file]").setInputFiles("tests/fixtures/sample.png");await page.getByLabel("Label *",{exact:true}).fill("Shutdown example");await page.getByRole("button",{name:"Run",exact:true}).click();
    await expect(page.getByRole("button",{name:"Cancel run"})).toBeVisible();
    await page.getByRole("button",{name:"Quit workspace",exact:true}).click();
    const drawer=page.getByRole("dialog",{name:"Quit workspace",exact:true});
    await expect(drawer.getByText(/1 active or unconfirmed/)).toBeVisible();
    await drawer.getByRole("button",{name:"Keep running"}).click();
    await expect(page.getByRole("button",{name:"Cancel run"})).toBeVisible();
    await page.getByRole("button",{name:"Quit workspace",exact:true}).click();
    await drawer.getByRole("button",{name:"Confirm quit"}).click();
    await expect(drawer.getByText("Run wafflehouse again to reopen the workspace.")).toBeVisible();
  });
});
test.describe("force shutdown",()=>{
  test.use({hostScenario:"ignore-eof"});
  test("force is a separate action after the tool refuses EOF",async({page,host})=>{
    await host.open(page);await page.getByRole("button",{name:"Quit workspace",exact:true}).click();
    const drawer=page.getByRole("dialog",{name:"Quit workspace",exact:true});
    await expect(drawer.getByRole("button",{name:"Force quit owned tools"})).toHaveCount(0);
    await drawer.getByRole("button",{name:"Confirm quit"}).click();
    await expect(drawer.getByRole("button",{name:"Force quit owned tools"})).toBeVisible({timeout:10000});
    await drawer.getByRole("button",{name:"Force quit owned tools"}).click();
    await expect(drawer.getByText("Run wafflehouse again to reopen the workspace.")).toBeVisible();
  });
});
