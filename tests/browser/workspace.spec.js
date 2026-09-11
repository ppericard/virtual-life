import { test, expect, openPanel, waitForGridFit } from './fixtures.js';

test.use({serverArgs:['--mode','autonomous','--width','64','--height','48','--ticks','160','--tick-ms','0']});

test('the complete world and exact counts fit the first desktop and narrow screen', async ({page,server},info) => {
  for(const size of [{width:1280,height:900},{width:1680,height:1000},{width:390,height:844}]) {
    await page.setViewportSize(size);
    await page.goto(server.url);
    await expect(page.locator('#tick')).toHaveText('0');
    await expect(page.getByRole('heading',{name:'VirtualLife',exact:true})).toBeVisible();
    await waitForGridFit(page);
    const box=await page.locator('#grid').boundingBox();
    expect(box.y).toBeLessThan(240);
    expect(box.y+box.height).toBeLessThanOrEqual(size.height);
    expect(box.x+box.width).toBeLessThanOrEqual(size.width);
    expect(box.width).toBeGreaterThan(250);
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    await expect(page.locator('#count')).toHaveText('921');
    await expect(page.locator('#population-groups .group-count')).toHaveText(['231','230','230','230']);
    for(const id of ['inspection-panel','new-experiment-panel','analysis-panel','details-panel']) await expect(page.locator(`#${id}`)).toBeHidden();
    await page.screenshot({path:info.outputPath(`world-first-${size.width}.png`),fullPage:true});
  }
});

async function save(page, info, name) {
  const path=info.outputPath(`${name}.png`);
  await page.screenshot({path,fullPage:true});
  await info.attach(name,{path,contentType:'image/png'});
}
async function clickSquare(page, sample, index) {
  await waitForGridFit(page);
  const box=await page.locator('#grid').boundingBox(), pitch=540/Math.max(sample.width,sample.height);
  const scale=box.width/(16+sample.width*pitch);
  await page.locator('#grid').click({position:{x:(8+(index%sample.width+.5)*pitch)*scale,y:(8+(Math.floor(index/sample.width)+.5)*pitch)*scale}});
}

test('keyboard and cell inspection, secondary panels and pending setup are read-only', async ({page,server},info) => {
  await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
  const before=await (await page.request.get(`${server.url}/api/snapshot`)).json();
  const occupied=before.cells.findIndex(Boolean), empty=before.cells.findIndex(a=>!a);
  let posts=0; page.on('request',r=>{if(r.method()==='POST')posts++;});
  await page.locator('#inspect-toggle').focus(); await page.keyboard.press('Enter');
  await expect(page.locator('#agent')).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(page.locator('#agent')).toHaveValue(before.cells[occupied].id);
  await page.getByRole('button',{name:'Close inspection',exact:true}).click();
  await expect(page.locator('#inspect-toggle')).toBeFocused();
  await expect(page.locator('#agent')).toHaveValue(before.cells[occupied].id);
  await expect(page.locator('#inspection-panel')).toBeHidden();
  await clickSquare(page,before,occupied);
  await expect(page.locator('#inspection-panel')).toBeVisible();
  await expect(page.locator('#inspection')).toContainText('Last selected action: Not yet acted');
  await save(page,info,'desktop-inspection');
  await page.getByRole('button',{name:'Close inspection',exact:true}).click();
  await expect(page.locator('#grid')).toBeFocused();
  await clickSquare(page,before,empty);
  await expect(page.locator('#agent')).toHaveValue('');
  await expect(page.locator('#inspection-panel')).toBeHidden();
  await clickSquare(page,before,occupied);
  for(const name of ['Analysis','Details','New experiment']) {
    await openPanel(page,name);
    await expect(page.locator('#agent')).toHaveValue(before.cells[occupied].id);
    if(name==='Analysis') {
      await expect(page.locator('#plot')).toBeVisible();
      await expect(page.locator('#samples')).toContainText('Tick 0 · 1 sample');
      await save(page,info,'desktop-analysis');
    }
    if(name==='Details') {
      await expect(page.locator('#configuration')).toContainText('wear-repair crowding v3');
      await expect(page.locator('#legend .group-row')).toHaveCount(4);
    }
    if(name==='New experiment') {
      await page.getByRole('button',{name:'Lower copying',exact:true}).click();
      await page.locator('#seed').fill('42');
      await save(page,info,'desktop-new-experiment');
    }
    await page.getByRole('button',{name:`Close ${name}`,exact:true}).click();
    await expect(page.getByRole('button',{name,exact:true})).toBeFocused();
  }
  await openPanel(page,'New experiment');
  await expect(page.locator('#seed')).toHaveValue('42');
  await expect(page.locator('#next-preset')).toContainText('Lower copying');
  await expect(page.locator('#current-preset')).toContainText('Original');
  await page.keyboard.press('Escape');
  await expect(page.locator('#new-experiment-toggle')).toBeFocused();
  await expect(page.locator('#new-experiment-panel')).toBeHidden();
  await page.getByRole('button',{name:'Close inspection',exact:true}).click();
  await page.setViewportSize({width:390,height:844});
  await clickSquare(page,before,occupied);
  await expect(page.locator('#inspection-panel')).toBeVisible();
  await expect(page.locator('#inspection')).toContainText(`ID ${before.cells[occupied].id}`);
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  await save(page,info,'narrow-inspection');
  await page.getByRole('button',{name:'Close inspection',exact:true}).click();
  await expect(page.locator('#grid')).toBeFocused();
  await openPanel(page,'New experiment');
  await save(page,info,'narrow-new-experiment');
  expect(posts).toBe(0);
  expect(await (await page.request.get(`${server.url}/api/snapshot`)).json()).toEqual(before);
});

test('narrow panel switches and desktop resizing keep requested controls reachable', async ({page,server},info) => {
  await page.setViewportSize({width:390,height:844});
  await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
  await page.locator('#step').click(); await expect(page.locator('#tick')).toHaveText('1');
  const before = await (await page.request.get(`${server.url}/api/snapshot`)).json();
  const selected = before.cells.find(Boolean).id;
  const history = await page.locator('#samples').textContent();
  let posts = 0; page.on('request', request => { if (request.method() === 'POST') posts++; });
  await openPanel(page,'Inspect'); await page.locator('#agent').selectOption(selected);
  for (const name of ['New experiment','Analysis','Details']) {
    await openPanel(page,name);
    const panel = page.locator(`#${await page.getByRole('button',{name,exact:true}).getAttribute('aria-controls')}`);
    await expect(panel).toBeFocused();
    if (name === 'New experiment') {
      await page.getByRole('button',{name:'Lower copying',exact:true}).click();
      await page.locator('#seed').fill('42');
      // A trial click runs the real actionability/hit test without restarting.
      await page.locator('#restart-seed').click({trial:true,timeout:2000});
      await page.locator('#restart-random').click({trial:true,timeout:2000});
      await save(page,info,'narrow-new-after-inspect');
    }
    await expect(page.locator('#inspection-panel')).toBeHidden();
    await expect(page.locator('#agent')).toHaveValue(selected);
    await expect(page.locator('#samples')).toHaveText(history);
    await openPanel(page,'Inspect');
    await expect(panel).toBeHidden();
    await expect(page.locator('#agent')).toBeFocused();
    await expect(page.locator('#agent')).toHaveValue(selected);
  }
  // Desktop supports both panels. On narrowing, the most recently requested
  // panel remains open and focus must leave any panel that becomes hidden.
  await page.setViewportSize({width:1280,height:900});
  await openPanel(page,'New experiment');
  await expect(page.locator('#inspection-panel')).toBeVisible();
  await expect(page.locator('#seed')).toHaveValue('42');
  await page.locator('#agent').focus();
  await page.setViewportSize({width:390,height:844});
  await expect(page.locator('#inspection-panel')).toBeHidden();
  await expect(page.locator('#new-experiment-panel')).toBeVisible();
  await expect(page.locator('#inspect-toggle')).toBeFocused();
  await page.locator('#restart-seed').click({trial:true});
  await page.setViewportSize({width:1280,height:900});
  await openPanel(page,'Inspect');
  await expect(page.locator('#new-experiment-panel')).toBeVisible();
  await page.locator('#seed').focus();
  await page.setViewportSize({width:390,height:844});
  await expect(page.locator('#new-experiment-panel')).toBeHidden();
  await expect(page.locator('#inspection-panel')).toBeVisible();
  await expect(page.locator('#new-experiment-toggle')).toBeFocused();
  await openPanel(page,'New experiment');
  await expect(page.locator('#seed')).toHaveValue('42');
  await expect(page.locator('#next-preset')).toContainText('Lower copying');
  await expect(page.locator('#agent')).toHaveValue(selected);
  await expect(page.locator('#samples')).toHaveText(history);
  expect(posts).toBe(0);
  expect(await (await page.request.get(`${server.url}/api/snapshot`)).json()).toEqual(before);
});

for (const width of [1280,390]) {
  test(`mode changes reconcile unavailable panels and visible focus at ${width}px`, async ({page,context,server},info) => {
    await page.setViewportSize({width,height:844});
    await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
    const initial = await (await page.request.get(`${server.url}/api/snapshot`)).json();
    const panel = page.locator('#new-experiment-panel'), toggle = page.locator('#new-experiment-toggle');
    let posts = 0; page.on('request', request => { if (request.method() === 'POST') posts++; });
    // Cover a retained input, a preset removed during run reconciliation, the
    // disappearing trigger, and unrelated focus that must not be stolen.
    for (const focused of ['#seed','[data-preset="lower-copying"]','#new-experiment-toggle','#details-toggle']) {
      await openPanel(page,'New experiment');
      await page.getByRole('button',{name:'Lower copying',exact:true}).click();
      await page.locator('#seed').fill('42');
      await page.locator(focused).focus();
      const previousRun = (await page.request.get(`${server.url}/api/snapshot`)).headers()['x-virtuallife-run'];
      if (focused === '#seed') {
        // Exercise focus already lost when disconnection disables the input.
        await context.setOffline(true); await expect(page.locator('#connection')).toBeVisible();
      }
      await server.restart(['--mode','demo']);
      await context.setOffline(false);
      await expect(toggle).toBeHidden();
      await expect(page.locator('#count')).toHaveText('3');
      await expect(page.locator('#run-message')).toContainText('New experiment connected');
      await expect(panel).toBeHidden();
      await expect(toggle).toHaveAttribute('aria-expanded','false');
      await expect(page.locator('#restart-controls')).toBeHidden();
      const focus = page.locator(focused === '#details-toggle' ? focused : '#inspect-toggle');
      await expect(focus).toBeVisible(); await expect(focus).toBeFocused();
      await expect(page.locator('#agent')).toHaveValue('');
      await expect(page.locator('#samples')).toContainText('Tick 0 · 1 sample');
      const demo = await page.request.get(`${server.url}/api/snapshot`);
      expect(demo.headers()['x-virtuallife-run']).not.toBe(previousRun);
      expect((await demo.json()).experiment).toBeNull();
      if (focused === '#seed') await save(page,info,`mode-change-${width}`);
      await server.restart(); // Return to the fixture's original autonomous settings.
      await expect(toggle).toBeVisible();
      await expect(page.locator('#count')).toHaveText(initial.count);
      await expect(panel).toBeHidden();
      await expect(toggle).toHaveAttribute('aria-expanded','false');
      await expect(focus).toBeFocused();
      await openPanel(page,'New experiment');
      await expect(page.locator('#seed')).toHaveValue('1');
      await expect(page.locator('#next-preset')).toContainText('Original');
      await page.locator('#restart-seed').click({trial:true});
      await page.locator('#restart-random').click({trial:true});
      await page.getByRole('button',{name:'Close New experiment',exact:true}).click();
      await expect(toggle).toBeVisible(); await expect(toggle).toBeFocused();
      expect(await (await page.request.get(`${server.url}/api/snapshot`)).json()).toEqual(initial);
    }
    expect(posts).toBe(0);
  });
}

test.describe('hidden observation with eight property groups',()=>{
  test.use({serverArgs:['--mode','autonomous','--width','5','--height','5','--occupancy','1','--bundles','1,0,0,0;2,0,0,0;3,0,0,0;4,0,0,0;5,0,0,0;6,0,0,0;7,0,0,0;8,0,0,0','--proportions','1,1,1,1,1,1,1,0','--upkeep','0','--crowding-upkeep','0','--ticks','136','--tick-ms','0']});
  test('closed analysis retains bounded history and reopens while paused or offline',async({page,context,server},info)=>{
    test.setTimeout(60000);
    await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
    await expect(page.locator('#population-groups .group-count')).toHaveText(['4','4','4','4','3','3','3','0']);
    await expect(page.locator('#population-groups .group-symbol')).toHaveText(['A','B','C','D','E','F','G','H']);
    await openPanel(page,'Inspect'); await page.locator('#agent').selectOption('1');
    await page.getByRole('button',{name:'Close inspection',exact:true}).click();
    for(let tick=1;tick<=136;tick++) {
      await page.locator('#step').click(); await expect(page.locator('#tick')).toHaveText(String(tick));
    }
    await expect(page.locator('#analysis-panel')).toBeHidden();
    await expect(page.locator('#samples')).toContainText('Ticks 9–136 · 128 samples · 0 sampling gaps. Older samples are discarded');
    const before=await (await page.request.get(`${server.url}/api/snapshot`)).json();
    const history=await page.locator('#samples').textContent();
    let posts=0; page.on('request',r=>{if(r.method()==='POST')posts++;});
    await openPanel(page,'Analysis');
    const pixels=await page.locator('#plot').evaluate(c=>c.toDataURL());
    await expect(page.locator('#plot')).toHaveAttribute('aria-label',/Tick 136:.*Group H 0; total 25/);
    await save(page,info,'retained-analysis');
    await page.getByRole('button',{name:'Close Analysis',exact:true}).click();
    await context.setOffline(true); await expect(page.locator('#connection')).toBeVisible();
    await openPanel(page,'Analysis');
    await expect(page.locator('#samples')).toHaveText(history);
    await expect.poll(()=>page.locator('#plot').evaluate(c=>c.toDataURL())).toBe(pixels);
    await page.setViewportSize({width:390,height:844});
    await expect(page.locator('#samples')).toHaveText(history);
    await expect(page.locator('#agent')).toHaveValue('1');
    await expect.poll(()=>page.locator('#plot').evaluate(c=>c.width===Math.round(c.getBoundingClientRect().width*devicePixelRatio))).toBe(true);
    await save(page,info,'narrow-analysis-offline');
    await context.setOffline(false); await expect(page.locator('#connection')).toBeHidden();
    await expect(page.locator('#samples')).toHaveText(history);
    expect(posts).toBe(0);
    expect(await (await page.request.get(`${server.url}/api/snapshot`)).json()).toEqual(before);
  });
});
