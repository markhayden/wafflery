import {test,expect} from "./tool-host";
import {visualPolicy} from "./visual-policy";
test.describe("live operations hierarchy",()=>{
  test.use({hostScenario:"v04-operations"});
  test("kinds and groups use declarations and changing activity resets the draft",async({page,host})=>{
    await host.open(page);
    await expect(page.getByRole("button",{name:"Workspace Playground",exact:true})).toBeVisible();
    await page.getByRole("button",{name:"Workspace Playground",exact:true}).click();
    await page.getByRole("dialog",{name:"Choose workspace"}).getByRole("button",{name:"Operations",exact:true}).click();
    await page.getByRole("button",{name:/^Operation locate marker/}).click();
    const picker=page.getByRole("dialog",{name:"Choose operation"});
    await expect(picker.getByText("locate",{exact:true})).toBeVisible();
    await expect(picker.getByText("shape",{exact:true})).toBeVisible();
    await picker.getByRole("button",{name:/^soften token/}).click();
    const amount=page.getByLabel("amount",{exact:true});
    await amount.fill("0.8");
    await page.getByRole("button",{name:"Workspace Operations",exact:true}).click();
    await page.getByRole("dialog",{name:"Choose workspace"}).getByRole("button",{name:"Utilities",exact:true}).click();
    await expect(amount).toHaveCount(0);
    await expect(page.getByRole("button",{name:/^Utility summarize/})).toBeVisible();
    await page.getByRole("button",{name:"Workspace Utilities",exact:true}).click();
    await page.getByRole("dialog",{name:"Choose workspace"}).getByRole("button",{name:"Operations",exact:true}).click();
    await page.getByRole("button",{name:/^Operation locate marker/}).click();
    await picker.getByRole("button",{name:/^soften token/}).click();
    await expect(amount).toHaveValue("0.25");
  });
});
for(const width of [1440,390]) {
  test(`grouped activity picker keyboard and layout at ${width}`,async({page,browserName})=>{
    await page.setViewportSize({width,height:900});
    await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-activitypicker--grouped&viewMode=story");
    const operation=page.getByRole("button",{name:/^Operation Measure/});
    await operation.focus();await page.keyboard.press("Enter");
    const dialog=page.getByRole("dialog",{name:"Choose operation"});
    await expect(dialog.getByRole("textbox")).toBeFocused();
    await expect(dialog.getByRole("button").filter({has:page.locator("strong")})).toHaveText([
      /Build a shape/,/Refine a shape/,/Measure a boundary/]);
    await dialog.getByRole("textbox").fill("Refine");
    await page.keyboard.press(browserName==="webkit" && process.platform==="darwin"?"Alt+Tab":"Tab");
    await page.keyboard.press("Enter");
    await expect(page.getByRole("button",{name:/^Operation Refine/})).toBeFocused();
    await page.getByRole("button",{name:/^Operation Refine/}).click();
    await dialog.getByRole("heading").click();await page.mouse.move(0,0);
    await expect(dialog).toHaveScreenshot(`activity-menu-${width}.png`,visualPolicy);
    await page.keyboard.press("Escape");
    for(const story of ["empty-kind","long-labels","task-only","needs-setup"]) {
      await page.goto(`http://127.0.0.1:6006/iframe.html?id=workspace-activitypicker--${story}&viewMode=story`);
      const root=page.locator("#storybook-root");
      if(story==="empty-kind") await expect(page.getByRole("status")).toContainText("No activities");
      if(story==="needs-setup") await expect(page.getByRole("status")).toContainText("Install the marker engine");
      if(story==="task-only") await expect(page.getByRole("button",{name:/^Workspace/})).toHaveCount(0);
      await expect(root).toHaveScreenshot(`activity-${story}-${width}.png`,visualPolicy);
      expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
    }
  });
}
for(const width of [390,1440])for(const task of [false,true])test(`purpose prose stays in the chooser while ${task?"task":"operation"} controls remain aligned at ${width}`,async({page})=>{
 await page.setViewportSize({width,height:900});await page.goto(`http://127.0.0.1:6006/iframe.html?id=workspace-activitypicker--${task?"long-task-purpose":"long-purpose"}&viewMode=story`);
 const activity=page.getByRole("button",{name:task?"Action Repair a sample":"Operation Refine mask",exact:true}),workspace=page.getByRole("button",{name:task?"Workspace Playground":"Workspace Operations",exact:true});
 await expect(activity).toContainText(task?"Image → Result":"rascal:Mask → rascal:Mask");expect((await activity.boundingBox())!.height).toBe((await workspace.boundingBox())!.height);
 await expect(page.locator("#storybook-root")).toHaveScreenshot(`activity-long-${task?"task":"purpose"}-${width}.png`,visualPolicy);
 await activity.click();await expect(page.getByRole("dialog",{name:task?"Choose action":"Choose operation"})).toContainText(task?"A complete sample task with detailed purpose.":"Refine the selected mask independently.");
});
