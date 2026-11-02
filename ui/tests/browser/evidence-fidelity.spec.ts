import { test, expect } from './fixtures';
import { visualPolicy } from './visual-policy';
for (const width of [1440,390]) {
  test(`evidence navigation stays reachable at ${width}px`, async ({page,browserName}) => {
    await page.setViewportSize({width,height:900});
    await page.goto('http://127.0.0.1:6006/iframe.html?id=workspace-evidence--run-details&viewMode=story');
    const nav=page.getByRole('navigation',{name:'Run evidence'});
    const first=nav.getByRole('button',{name:'Saved inputs'});
    const last=nav.getByRole('button',{name:'Reports',exact:true});
    await expect(first).toBeVisible();
    expect((await first.boundingBox())!.y).toBe((await last.boundingBox())!.y);
    await first.focus();
    for(let i=0;i<5;i++) await page.keyboard.press(browserName==='webkit' && process.platform==='darwin'?'Alt+Tab':'Tab');
    await expect(last).toBeFocused();
    await expect(last).toBeInViewport();
    await first.focus();
    await page.getByRole('heading',{name:'Run details',exact:true}).click();
    await page.mouse.move(0,0);
    await expect(page.getByRole('dialog')).toHaveScreenshot(`inspector-${width}.png`,visualPolicy);
    await page.goto('http://127.0.0.1:6006/iframe.html?id=workspace-evidence--use-in-pipeline&viewMode=story');
    await expect(page.getByRole('navigation',{name:'Pipeline views'})).toBeVisible();
    await page.getByRole('heading',{name:'Use in your pipeline',exact:true}).click();
    await page.mouse.move(0,0);
    await expect(page.getByRole('dialog')).toHaveScreenshot(`pipeline-${width}.png`,visualPolicy);
    expect(await page.locator('dialog[open]').evaluate(node=>node.scrollWidth<=node.clientWidth)).toBe(true);
  });
}
