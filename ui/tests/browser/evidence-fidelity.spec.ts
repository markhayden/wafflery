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
    for(let i=0, steps=await nav.getByRole('button').count()-1;i<steps;i++) await page.keyboard.press(browserName==='webkit' && process.platform==='darwin'?'Alt+Tab':'Tab');
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
    const pipeline=page.getByRole('navigation',{name:'Pipeline views'});
    await pipeline.getByRole('button',{name:'Recorded MCP call'}).focus();
    for(let i=0;i<2;i++) await page.keyboard.press(browserName==='webkit' && process.platform==='darwin'?'Alt+Tab':'Tab');
    await expect(pipeline.getByRole('button',{name:'Evidence archive'})).toBeFocused();
    await expect(pipeline.getByRole('button',{name:'Evidence archive'})).toBeInViewport();
    expect(await page.locator('dialog[open]').evaluate(node=>node.scrollWidth<=node.clientWidth)).toBe(true);
  });
}

test('drawer close control scales with larger text', async ({page}) => {
  await page.goto('http://127.0.0.1:6006/iframe.html?id=workspace-evidence--run-details&viewMode=story');
  await page.addStyleTag({content:':root { font-size: 24px; }'});
  const close=page.getByRole('button',{name:'Close',exact:true});
  await expect(close).toBeVisible();
  const box=(await close.boundingBox())!;
  expect(box.width).toBeGreaterThanOrEqual(66);
  expect(box.width).toBe(box.height);
});

for (const width of [1440,390]) {
  for (const [state, story, message] of [
    ['resolved','run-details','The tool’s resolved arguments, effective settings and requirement names for this saved run.'],
    ['unresolved','unresolved-invocation','The tool rejected the inputs before resolving an invocation.'],
    ['unavailable','unavailable-invocation','No resolved invocation is available for this saved attempt.'],
  ] as const) {
    test(`saved invocation ${state} at ${width}px`, async ({page}) => {
      await page.setViewportSize({width,height:900});
      await page.goto(`http://127.0.0.1:6006/iframe.html?id=workspace-evidence--${story}&viewMode=story`);
      const drawer=page.getByRole('dialog');
      await drawer.getByRole('button',{name:'Resolved invocation',exact:true}).click();
      await expect(drawer.getByText(message,{exact:true})).toBeVisible();
      const evidence=drawer.getByLabel('Resolved invocation',{exact:true});
      if(state==='resolved') await expect(evidence).toContainText('"strength": 0.5');
      if(state==='unresolved') await expect(evidence).toHaveText('null');
      if(state==='unavailable') await expect(evidence).toHaveCount(0);
      await drawer.getByRole('heading',{name:'Run details',exact:true}).click();
      await page.mouse.move(0,0);
      await expect(drawer).toHaveScreenshot(`invocation-${state}-${width}.png`,visualPolicy);
      expect(await drawer.evaluate(node=>node.scrollWidth<=node.clientWidth)).toBe(true);
    });
  }
}
