import { test, expect } from './fixtures.js';

const colors = ['#0072b2', '#d55e00', '#009e73', '#cc79a7'];
async function snapshot(page, server) { return (await page.request.get(`${server.url}/api/snapshot`)).json(); }
async function save(page, info, name) {
  const file = info.outputPath(`${name}.png`);
  await page.screenshot({path:file,fullPage:true});
  await info.attach(name,{path:file,contentType:'image/png'});
}

test.describe('autonomous visual experiment', () => {
  test.use({serverArgs:['--mode','autonomous','--width','8','--height','6','--seed','1','--ticks','80','--tick-ms','25']});
  test('property colours, grid click, population trajectories and long completion', async ({page,server}, info) => {
    await page.goto(server.url);
    await expect(page.locator('#tick')).toHaveText('0');
    await expect(page.locator('#end-tick')).toHaveText('80');
    await expect(page.locator('#dimensions')).toContainText('8 × 6');
    await expect(page.locator('#configuration')).toContainText('Seed 1 · SplitMix64 / VirtualLife sampling v1');
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
    await expect(page.locator('#samples')).toContainText('Observed ticks: 80.');
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
    await expect(page.locator('#samples')).toContainText('Observed ticks: 0.');
    await expect(page.locator('#agent')).toHaveValue('');
    await expect(page.getByRole('status')).toHaveText('paused');
    expect((await snapshot(page,server)).cells).toEqual(initial.cells);
  });
});

test.describe('canonical zero groups and extinction', () => {
  test.use({serverArgs:['--mode','autonomous','--width','8','--height','6','--bundles','0,0,0,1;0,0,0,1;1,0,0,0;0,1,0,0','--proportions','1,1,0,0','--ticks','20','--tick-ms','0']});
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
  test.use({serverArgs:['--mode','autonomous','--occupancy','0','--ticks','600','--running','--tick-ms','0']});
  test('empty completion retains configuration and all four zero series', async ({page,server}) => {
    await page.goto(server.url); await expect(page.getByRole('status')).toHaveText('completed');
    await expect(page.locator('#tick')).toHaveText('600'); await expect(page.locator('#count')).toHaveText('0');
    await expect(page.locator('#legend .group-count')).toHaveText(['0','0','0','0']);
    await expect(page.locator('#plot')).toHaveAttribute('aria-label',/Tick 600: Group A 0, Group B 0, Group C 0, Group D 0; total 0/);
    await expect(page.locator('#samples')).toContainText('not a complete recording');
  });
});
