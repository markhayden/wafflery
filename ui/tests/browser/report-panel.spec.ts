import { test, expect } from "./fixtures";
const story = (name:string) => `http://127.0.0.1:6006/iframe.html?id=workspace-reportpanel--${name}&viewMode=story`;

test("report force stop requires an explicit confirmation and uncertain opens stay blocked", async ({page}) => {
  await page.goto(story("needs-force"));
  const force = page.getByRole("button",{name:"Force stop report",exact:true});
  await force.focus(); await page.keyboard.press("Enter");
  await expect(page.getByRole("button",{name:"Keep report",exact:true})).toBeFocused();
  await page.getByRole("button",{name:"Keep report",exact:true}).click();
  await expect(page.getByText("Force stops confirmed: 0")).toBeVisible();
  await force.click();
  await page.getByRole("button",{name:"Confirm force stop",exact:true}).click();
  await expect(page.getByText("Force stops confirmed: 1")).toBeVisible();
  await expect(page.getByRole("button",{name:"Open report",exact:true})).toBeEnabled();
  await page.goto(story("unconfirmed"));
  await expect(page.getByRole("button",{name:"Open report",exact:true})).toBeDisabled();
  await expect(page.getByRole("alert")).toContainText("Quit and reopen Wafflery");
});

test("report states fit the shared drawer at narrow and wide sizes", async ({page}) => {
  test.setTimeout(30_000);
  for (const state of ["empty","starting","ready","failed","unavailable"]) {
    await page.goto(story(state));
    const report = page.getByRole("region",{name:"Tool report",exact:true});
    await expect(report).toBeVisible();
    for (const width of [320,768,1440]) {
      await page.setViewportSize({width,height:900});
      expect(await page.locator("dialog[open]").evaluate(node => node.scrollWidth <= node.clientWidth)).toBe(true);
      if (width !== 768) {
        await page.mouse.move(0,0);
        await expect(page).toHaveScreenshot(`report-${state}-${width}.png`,{animations:"disabled",maxDiffPixelRatio:.001});
      }
    }
  }
});
