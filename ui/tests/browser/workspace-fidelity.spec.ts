import { test, expect } from './tool-host';

for (const width of [1440, 390]) {
  test(`tool context precedes the action at ${width}px`, async ({page,host}) => {
    await page.setViewportSize({width,height:1000});
    await host.open(page);
    const header = page.locator('.workspace-header');
    const tool = header.getByRole('button',{name:'Tool sample-lab',exact:true});
    await expect(tool).toBeVisible();
    await expect(page.getByRole('heading',{name:'sample-lab',exact:true})).toBeVisible();
    const action = page.getByRole('button',{name:'Action Add a test band'});
    await expect(action).toBeVisible();
    expect((await tool.boundingBox())!.y).toBeLessThan((await action.boundingBox())!.y);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
    await tool.click();
    const dialog = page.getByRole('dialog',{name:'Choose tool'});
    await expect(dialog.getByRole('textbox',{name:'Search tool'})).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(tool).toBeFocused();
    // Remove hover/focus variability before comparing the rendered workspace.
    await page.getByRole('heading',{name:'sample-lab',exact:true}).click();
    await page.mouse.move(0,0);
    await expect(page).toHaveScreenshot(`workspace-${width}.png`,{fullPage:true,animations:'disabled',maxDiffPixelRatio:0.001});
  });
}
