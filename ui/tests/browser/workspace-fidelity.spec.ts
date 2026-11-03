import { test, expect } from './tool-host';
import { visualPolicy } from './visual-policy';

for (const width of [1440, 390]) {
  test(`tool context precedes the action at ${width}px`, async ({page,host,browserName}) => {
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
    const input = page.locator('main form').first();
    const result = page.getByRole('region',{name:'Result',exact:true});
    const inputBox = (await input.boundingBox())!;
    const resultBox = (await result.boundingBox())!;
    if (width > 960) {
      expect(inputBox.y).toBe(resultBox.y);
      expect(resultBox.width / inputBox.width).toBeCloseTo(1.5,1);
    } else expect(resultBox.y).toBeGreaterThan(inputBox.y + inputBox.height);
    const regions = {
      header,
      context: page.getByRole('heading',{name:'sample-lab',exact:true}).locator('../..'),
      action,
      input,
      result,
    };
    for (const [name,region] of Object.entries(regions)) {
      await expect(region).toHaveScreenshot(`workspace-${name}-${width}.png`,visualPolicy);
    }
    // Native gutters may be absent (overlay) or reserve space on another Mac.
    // Force an even wider classic gutter, then prove captures stay identical.
    if (width === 1440 && browserName === 'chromium') {
      const classic = await page.addStyleTag({content: `
        * { scrollbar-width: auto !important; }
        *::-webkit-scrollbar { width: 24px; height: 24px; }
      `});
      const settings = page.getByRole('region',{name:'Config settings'});
      expect(await settings.evaluate(node => (node as HTMLElement).offsetWidth - node.clientWidth)).toBe(24);
      for (const [name,region] of Object.entries(regions)) {
        await expect(region).toHaveScreenshot(`workspace-${name}-${width}.png`,visualPolicy);
      }
      await classic.evaluate(node => (node as HTMLElement).remove());
    }
    if (width === 1440) {
      // These use the same baseline and tolerance as normal captures. Negated
      // assertions never update a baseline, even during --update-snapshots.
      await action.evaluate(node => {
        (node as HTMLElement).style.paddingLeft = `${parseFloat(getComputedStyle(node).paddingLeft) + 24}px`;
      });
      await expect(action).not.toHaveScreenshot(`workspace-action-${width}.png`,visualPolicy);
      await action.evaluate(node => (node as HTMLElement).style.removeProperty('padding-left'));
      await action.evaluate(node => (node as HTMLElement).style.setProperty('--color-surface','#815161'));
      await expect(action).not.toHaveScreenshot(`workspace-action-${width}.png`,visualPolicy);
      await action.evaluate(node => (node as HTMLElement).style.removeProperty('--color-surface'));
      await expect(action).toHaveScreenshot(`workspace-action-${width}.png`,visualPolicy);
    }
  });
}
