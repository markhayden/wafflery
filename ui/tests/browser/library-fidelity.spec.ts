import { test, expect } from './fixtures';
import { visualPolicy } from './visual-policy';
for (const width of [320,1440]) {
  test(`example states stay truthful and readable at ${width}px`, async ({page}) => {
    test.setTimeout(30000);
    await page.setViewportSize({width,height:900});
    for (const state of ['saved','empty','loading','unavailable']) {
      await page.goto(`http://127.0.0.1:6006/iframe.html?id=workspace-examples--${state}&viewMode=story`);
      const drawer=page.getByRole('dialog',{name:'Examples',exact:true});
      await expect(drawer).toBeVisible();
      if(state==='saved') await expect(drawer.getByRole('button',{name:'Open A saved experiment with a longer name'})).toBeVisible();
      if(state==='empty') await expect(drawer.getByText('No examples saved yet.')).toBeVisible();
      if(state==='loading') {
        await expect(drawer.getByRole('status')).toHaveText('Loading examples…');
        await expect(drawer.getByText('No examples saved yet.')).not.toBeVisible();
      }
      if(state==='unavailable') {
        await expect(drawer.getByRole('alert')).toContainText('library is unavailable');
        await expect(drawer.getByText('No examples saved yet.')).not.toBeVisible();
      }
      expect(await drawer.evaluate(node=>node.scrollWidth<=node.clientWidth)).toBe(true);
      await expect(drawer).toHaveScreenshot(`examples-${state}-${width}.png`,visualPolicy);
    }
  });
  test(`comparison states preserve image meaning at ${width}px`, async ({page}) => {
    test.setTimeout(30000);
    await page.setViewportSize({width,height:900});
    for(const state of ['wipe','incompatible','missing','unavailable','loading']) {
      await page.goto(`http://127.0.0.1:6006/iframe.html?id=components-imagecompare--${state}&viewMode=story`);
      const view=page.getByRole('region',{name:'Image comparison'});
      await expect(view).toBeVisible();
      if(state==='wipe') await expect(view.getByRole('slider')).toBeVisible();
      if(state==='incompatible') await expect(view.getByRole('button',{name:'Wipe',exact:true})).toBeDisabled();
      if(state==='missing') await expect(view.getByText('Result not available.')).toBeVisible();
      if(state==='unavailable') await expect(view.getByText(/saved bytes could not be read/)).toHaveCount(2);
      if(state==='loading') await expect(view.getByText('Loading image…')).toHaveCount(2);
      for(const image of await view.getByRole('img').all()) await expect.poll(()=>image.evaluate((node:HTMLImageElement)=>node.complete && node.naturalWidth>0)).toBe(true);
      expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
      await expect(view).toHaveScreenshot(`compare-${state}-${width}.png`,visualPolicy);
    }
  });
}

for (const width of [320,1440]) {
  test(`example errors stay with their actions at ${width}px`, async ({page}) => {
    await page.setViewportSize({width,height:900});
    await page.goto('http://127.0.0.1:6006/iframe.html?id=workspace-examples--action-failures&viewMode=story');
    const drawer=page.getByRole('dialog',{name:'Examples',exact:true});
    const entry=drawer.getByRole('listitem').first();
    await entry.getByRole('button').click();
    await expect(entry.getByRole('alert')).toContainText('saved inputs could not be read');
    const form=drawer.locator('form');
    await form.getByLabel('Example name').fill('Another example');
    await form.getByRole('button',{name:'Save example'}).click();
    await expect(form.getByRole('alert')).toContainText('could not be saved');
    await expect(form.getByRole('alert')).toBeInViewport();
    await expect(form).toHaveScreenshot(`examples-save-failure-${width}.png`,visualPolicy);
    await page.goto('http://127.0.0.1:6006/iframe.html?id=workspace-examples--empty-save-failure&viewMode=story');
    await form.getByLabel('Example name').fill('Another example');
    await form.getByRole('button',{name:'Save example'}).click();
    await expect(form.getByRole('alert')).toContainText('could not be saved');
    await expect(drawer.getByText('No examples saved yet.')).toBeVisible();
    await page.goto('http://127.0.0.1:6006/iframe.html?id=workspace-examples--unavailable&viewMode=story');
    await expect(drawer.getByRole('alert')).toContainText('library is unavailable');
    await form.getByLabel('Example name').fill('Another example');
    await form.getByRole('button',{name:'Save example'}).click();
    await expect(form.getByRole('status')).toHaveText('Example saved. The list could not be refreshed.');
    await expect(form.getByRole('alert')).toHaveCount(0);
    await expect(form.getByLabel('Example name')).toHaveValue('');
    await expect(drawer.getByRole('alert')).toHaveCount(1);
    await expect(drawer.getByText('No examples saved yet.')).not.toBeVisible();
  });
}
