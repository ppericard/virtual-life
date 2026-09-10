import { test, expect } from './fixtures.js';

const colors = ['#0072b2', '#d55e00', '#009e73', '#cc79a7'];
async function snapshot(page, server) { return (await page.request.get(`${server.url}/api/snapshot`)).json(); }
async function save(page, info, name) {
  const file = info.outputPath(`${name}.png`);
  await page.screenshot({path:file,fullPage:true});
  await info.attach(name,{path:file,contentType:'image/png'});
}

test.describe('wear and repair default experiment', () => {
  test.use({serverArgs:['--mode','autonomous','--width','8','--height','6','--ticks','80','--tick-ms','0']});
  test('live integrity recovers, properties remain stable and completion keeps failure evidence', async ({page,server},info) => {
    await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
    await expect(page.locator('#action-order')).toHaveText('Wait / move / copy / repair');
    await expect(page.locator('#configuration')).toContainText('Maximum, initial and newborn integrity 10');
    await expect(page.locator('#configuration')).toContainText('wear-repair crowding v2');
    await expect(page.locator('#choice-rule')).toContainText('Copy chance is scaled by the fraction of empty neighbours');
    let previous=await snapshot(page,server);
    expect(previous.experiment.survival).toBe('wear-repair');
    expect(previous.experiment.protocol).toBe('wear-repair crowding v2');
    expect(previous.experiment.groups.map(g=>g.weights)).toEqual([[2,4,1,3],[2,2,2,4],[4,1,1,4],[1,5,2,2]]);
    const selected=previous.cells.find(Boolean); await page.locator('#agent').selectOption(selected.id);
    await expect(page.locator('#inspection')).toContainText('Integrity 10/10');
    await save(page,info,'wear-repair-initial');
    let recovered=false;
    for(let tick=1;tick<=10;tick++) {
      await page.getByRole('button',{name:'Single step'}).click(); await expect(page.locator('#tick')).toHaveText(String(tick));
      const current=await snapshot(page,server);
      const recovering=current.cells.filter(Boolean).find(agent=>previous.cells.some(old=>old?.id===agent.id && old.integrity<agent.integrity));
      if(recovering&&!recovered) {
        await page.locator('#agent').selectOption(recovering.id);
        await expect(page.locator('#inspection')).toContainText(`Integrity ${recovering.integrity}/10`);
        expect(previous.cells.find(agent=>agent?.id===recovering.id).weights).toEqual(recovering.weights);
        await save(page,info,'wear-repair-recovery'); recovered=true;
      }
      previous=current;
    }
    expect(recovered).toBe(true); expect(Number(previous.totals.repairs)).toBeGreaterThan(0);
    expect(previous.experiment.groups.map(g=>g.count)).toEqual(['4','10','6','1']);
    await page.locator('#agent').selectOption('23');
    await expect(page.locator('#inspection')).toHaveText('ID 23 · Group A · Base weights (wait, move, copy, repair): 2, 4, 1, 3 · Position (3, 0) · Integrity 7/10');
    await expect(page.locator('#plot')).toHaveAttribute('aria-label',/Tick 10: Group A 4, Group B 10, Group C 6, Group D 1; total 21/);
    await save(page,info,'wear-crowding-v2-tick10');
    await page.getByRole('button',{name:'Resume',exact:true}).click(); await expect(page.getByRole('status')).toHaveText('completed');
    const final=await snapshot(page,server);
    expect(final.experiment.groups.reduce((n,g)=>n+Number(g.count),0)).toBe(Number(final.count));
    expect(final.failure_history.records.length).toBeLessThanOrEqual(128);
    expect(Number(final.failure_history.discarded)+final.failure_history.records.length).toBe(Number(final.totals.failures));
    const failure=final.failure_history.records.at(-1);
    expect(failure).toBeTruthy(); await page.locator('#agent').selectOption(failure.id);
    await expect(page.locator('#inspection')).toContainText(`failed at tick ${failure.tick} · ${failure.reason}`);
    await save(page,info,'wear-repair-completed');
    await page.setViewportSize({width:390,height:950});
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth)).toBe(true);
    await expect(page.locator('#inspection')).toBeVisible();
    await save(page,info,'wear-repair-narrow');
    await page.reload(); await expect(page.getByRole('status')).toHaveText('completed');
    await expect(page.locator('#failure-summary')).toContainText(`${final.totals.failures} cumulative failures`);
    await expect(page.locator('#samples')).toContainText('Tick 80 · 1 sample');
    await server.restart(); await expect(page.locator('#run-message')).toContainText('New experiment connected');
    await expect(page.locator('#tick')).toHaveText('0'); await expect(page.locator('#agent')).toHaveValue('');
    await expect(page.locator('#failure-summary')).toContainText('0 cumulative failures');
    await expect(page.getByRole('status')).toHaveText('paused');
  });
});

test.describe('exact upkeep failure evidence', () => {
  test.use({serverArgs:['--mode','autonomous','--width','3','--height','3','--occupancy','1','--bundles','1,0,0,0','--proportions','1','--integrity','3','--ticks','3']});
  test('selected individual shows exact-zero upkeep failure at completion', async ({page,server},info) => {
    await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
    await page.locator('#agent').selectOption('1');
    for(let tick=1;tick<=2;tick++) {
      await page.getByRole('button',{name:'Single step'}).click(); await expect(page.locator('#tick')).toHaveText(String(tick));
      await expect(page.locator('#inspection')).toContainText(`Integrity ${3-tick}/3`);
    }
    await page.getByRole('button',{name:'Single step'}).click(); await expect(page.locator('#tick')).toHaveText('3');
    await expect(page.locator('#inspection')).toHaveText('Agent 1 failed at tick 3 · upkeep · Position (0, 0) · starting integrity 1, upkeep 1, extra wear 0.');
    const final=await snapshot(page,server); expect(final.count).toBe('0'); expect(final.totals.failures).toBe('9');
    expect(final.failure_history.records.map(f=>f.id)).toEqual(['1','2','3','4','5','6','7','8','9']);
    await save(page,info,'wear-repair-upkeep-failure');
  });
});

test.describe('bounded failures independent of sampling', () => {
  test.use({serverArgs:['--mode','autonomous','--width','20','--height','20','--occupancy','1','--integrity','1','--ticks','600','--sample-every','1000','--tick-ms','0']});
  test('discarded causes are labelled while retained outcomes survive sparse observation', async ({page,server},info) => {
    await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
    await page.locator('#agent').selectOption('1');
    await page.getByRole('button',{name:'Resume',exact:true}).click(); await expect(page.getByRole('status')).toHaveText('completed');
    await expect(page.locator('#inspection')).toContainText('record has been discarded');
    await expect(page.locator('#failure-summary')).toContainText('272 older records discarded');
    const final=await snapshot(page,server);
    expect(final.failure_history.records).toHaveLength(128); expect(final.failure_history.records[0].id).toBe('273');
    expect(final.failure_history.records.every(f=>f.tick==='1'&&f.reason==='upkeep')).toBe(true);
    await page.locator('#agent').selectOption('400'); await expect(page.locator('#inspection')).toContainText('failed at tick 1 · upkeep');
    await expect(page.locator('#samples')).toContainText('not a complete recording');
    await page.getByText('Recent failure records',{exact:true}).click();
    await expect(page.locator('#failure-list li')).toHaveCount(128);
    await save(page,info,'wear-repair-bounded-failures');
  });
});

test.describe('autonomous visual experiment', () => {
  test.use({serverArgs:['--mode','autonomous','--survival','random','--width','8','--height','6','--seed','1','--ticks','80','--tick-ms','25']});
  test('property colours, grid click, population trajectories and long completion', async ({page,server}, info) => {
    await page.goto(server.url);
    await expect(page.locator('#tick')).toHaveText('0');
    await expect(page.locator('#end-tick')).toHaveText('80');
    await expect(page.locator('#dimensions')).toContainText('8 × 6');
    await expect(page.locator('#configuration')).toContainText('Seed 1 · SplitMix64 / VirtualLife sampling v1');
    await expect(page.locator('#configuration')).toContainText('random v1');
    await expect(page.locator('#choice-rule')).toBeHidden();
    await expect(page.locator('#legend .group-row')).toHaveCount(4);
    const initial = await snapshot(page,server);
    expect(initial.count).toBe('14');
    expect(initial.experiment.groups.map(g=>g.initial_count)).toEqual(['4','4','3','3']);
    const pixels = await page.locator('#grid').evaluate(canvas => {
      const ctx=canvas.getContext('2d');
      return Array.from({length:48},(_,i)=>'#'+[...ctx.getImageData(40+i%8*67.5+5,40+Math.floor(i/8)*90+5,1,1).data].slice(0,3).map(n=>n.toString(16).padStart(2,'0')).join(''));
    });
    expect(pixels).toEqual(initial.cells.map(agent=>agent?colors[agent.group]:'#f5f7f2'));
    const index=initial.cells.findIndex(Boolean), agent=initial.cells[index];
    const canvas=page.locator('#grid'), box=await canvas.boundingBox();
    await canvas.click({position:{x:(40+(index%8+.5)*67.5)*box.width/600,y:(40+(Math.floor(index/8)+.5)*90)*box.height/600}});
    await expect(page.locator('#inspection')).toHaveText(`ID ${agent.id} · Group ${String.fromCharCode(65+agent.group)} · Weights (wait, move, copy, remove): ${agent.weights.join(', ')} · Position (${index%8}, ${Math.floor(index/8)})`);
    await save(page,info,'autonomous-initial-inspection');
    // Ten explicit steps are deterministic barriers and give adjacent plot samples.
    for(let tick=1;tick<=10;tick++) {
      await page.getByRole('button',{name:'Single step'}).click();
      await expect(page.locator('#tick')).toHaveText(String(tick));
    }
    const tenth=await snapshot(page,server);
    expect(tenth.count).toBe('9'); expect(tenth.experiment.groups.map(g=>g.count)).toEqual(['2','6','0','1']);
    await expect(page.locator('#plot')).toHaveAttribute('aria-label',/Tick 10: Group A 2, Group B 6, Group C 0, Group D 1; total 9/);
    await save(page,info,'autonomous-trajectories');
    await page.getByRole('button',{name:'Resume',exact:true}).click();
    await expect(page.getByRole('status')).toHaveText('completed');
    await expect(page.locator('#tick')).toHaveText('80');
    const final=await snapshot(page,server);
    expect(final.experiment.groups.reduce((sum,g)=>sum+Number(g.count),0)).toBe(Number(final.count));
    await expect(page.locator('#legend .group-count')).toHaveText(final.experiment.groups.map(g=>g.count));
    await expect(page.locator('#plot')).toHaveAttribute('aria-label',/Tick 80:/);
    for(const name of ['Resume','Pause','Single step']) await expect(page.getByRole('button',{name,exact:true})).toBeDisabled();
    await save(page,info,'autonomous-completed');
    await page.reload();
    await expect(page.locator('#samples')).toContainText('Tick 80 · 1 sample');
    await expect(page.locator('#legend .group-count')).toHaveText(final.experiment.groups.map(g=>g.count));
  });

  test('new autonomous experiment clears history and selection without retrying controls', async ({page,server}) => {
    await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
    const initial=await snapshot(page,server), agent=initial.cells.find(Boolean);
    await page.locator('#agent').selectOption(agent.id);
    await page.getByRole('button',{name:'Single step'}).click(); await expect(page.locator('#tick')).toHaveText('1');
    await server.restart();
    await expect(page.locator('#run-message')).toContainText('New experiment connected');
    await expect(page.locator('#tick')).toHaveText('0');
    await expect(page.locator('#samples')).toContainText('Tick 0 · 1 sample');
    await expect(page.locator('#agent')).toHaveValue('');
    await expect(page.getByRole('status')).toHaveText('paused');
    expect((await snapshot(page,server)).cells).toEqual(initial.cells);
  });
});

test.describe('canonical zero groups and extinction', () => {
  test.use({serverArgs:['--mode','autonomous','--survival','random','--width','8','--height','6','--bundles','0,0,0,1;0,0,0,1;1,0,0,0;0,1,0,0','--proportions','1,1,0,0','--ticks','20','--tick-ms','0']});
  test('duplicate bundles aggregate and every extinct/zero series retains its label', async ({page,server},info) => {
    await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
    await expect(page.locator('#legend .group-row')).toHaveCount(3);
    await expect(page.locator('#legend .group-count')).toHaveText(['14','0','0']);
    await expect(page.locator('#legend .group-row').first()).toContainText('ratio 2 · initially 14');
    await page.getByRole('button',{name:'Single step'}).click();
    await expect(page.locator('#tick')).toHaveText('1'); await expect(page.locator('#count')).toHaveText('0');
    await expect(page.locator('#legend .group-count')).toHaveText(['0','0','0']);
    await expect(page.locator('#plot')).toHaveAttribute('aria-label',/Tick 1: Group A 0, Group B 0, Group C 0; total 0/);
    await save(page,info,'autonomous-extinct');
    await page.getByRole('button',{name:'Resume',exact:true}).click(); await expect(page.getByRole('status')).toHaveText('completed');
    await expect(page.locator('#tick')).toHaveText('20');
    const final=await snapshot(page,server); expect(final.count).toBe('0'); expect(final.totals.removals).toBe('14');
    expect(final.experiment.groups.map(g=>g.weights)).toEqual([[0,0,0,1],[1,0,0,0],[0,1,0,0]]);
  });
});

test.describe('initially empty autonomous run', () => {
  test.use({serverArgs:['--mode','autonomous','--survival','random','--occupancy','0','--ticks','600','--running','--tick-ms','0']});
  test('empty completion retains configuration and all four zero series', async ({page,server}) => {
    await page.goto(server.url); await expect(page.getByRole('status')).toHaveText('completed');
    await expect(page.locator('#tick')).toHaveText('600'); await expect(page.locator('#count')).toHaveText('0');
    await expect(page.locator('#legend .group-count')).toHaveText(['0','0','0','0']);
    await expect(page.locator('#plot')).toHaveAttribute('aria-label',/Tick 600: Group A 0, Group B 0, Group C 0, Group D 0; total 0/);
    await expect(page.locator('#samples')).toContainText('not a complete recording');
  });
});
