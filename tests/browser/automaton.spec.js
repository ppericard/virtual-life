import {test, expect, inspectAgent, openPanel} from './fixtures.js';

test.use({serverArgs:['--mode','autonomous','--automaton-preset','mixed','--width','12','--height','9','--ticks','8','--tick-ms','60000']});
async function snapshot(page,server) {return (await page.request.get(`${server.url}/api/snapshot`)).json();}
test('graphs show actual probabilities, preserve observation and replay through preset restart',async({page,server},info)=> {
  await page.goto(server.url);
  const initial=await snapshot(page,server);
  expect(initial.experiment.protocol).toBe('unit-action automaton v1');
  expect(initial.experiment.groups).toHaveLength(4);
  const agent=initial.cells.find(a=>a?.group===0);
  await inspectAgent(page,agent.id);
  await expect(page.locator('#inspection')).toContainText('Movement runs · State Wait');
  await expect(page.locator('#inspection')).not.toContainText('Base weights');
  const graph=page.locator('#agent-automaton svg');
  await expect(graph).toHaveAttribute('aria-label',/Wait to Move: .*Wait to Repair:/);
  let writes=0; page.on('request',r=> {if(r.method()==='POST') writes++;});
  await page.getByLabel('Movement runs transitions').selectOption('Copy');
  await expect(graph).toHaveAttribute('aria-label','Copy to Repair: weight 1');
  await page.waitForResponse('**/api/snapshot');
  expect(await snapshot(page,server)).toEqual(initial);
  expect(writes).toBe(0);
  await page.getByLabel('Movement runs transitions').selectOption('current');
  await page.screenshot({path:info.outputPath('automaton-inspector.png'),fullPage:true});
  await page.locator('#step').click(); await expect(page.locator('#tick')).toHaveText('1');
  const first=await snapshot(page,server);
  await openPanel(page,'New experiment');
  await page.getByRole('button',{name:'Copy bursts',exact:true}).click();
  await page.locator('#preset-controls summary').click();
  await expect(page.locator('#preset-automata svg')).toHaveCount(1);
  await page.getByLabel('Group A automaton transitions').selectOption('Copy');
  await expect(page.locator('#preset-automata svg')).toHaveAttribute('aria-label',/Copy to Copy: weight 3/);
  await page.setViewportSize({width:390,height:844});
  await expect(page.locator('#preset-automata svg')).toBeVisible();
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  await page.locator('#preset-automata svg').scrollIntoViewIfNeeded();
  await page.screenshot({path:info.outputPath('automaton-preset-narrow.png'),fullPage:true});
  await page.getByRole('button',{name:'Mixed automata',exact:true}).click();
  await page.locator('#restart-seed').click(); await expect(page.locator('#tick')).toHaveText('0');
  expect(await snapshot(page,server)).toEqual(initial);
  await page.locator('#step').click(); await expect(page.locator('#tick')).toHaveText('1');
  expect(await snapshot(page,server)).toEqual(first);
  await page.getByRole('button',{name:'Copy bursts',exact:true}).click();
  await page.locator('#restart-seed').click(); await expect(page.locator('#tick')).toHaveText('0');
  expect((await snapshot(page,server)).experiment.groups).toHaveLength(1);
  await page.getByRole('button',{name:'Original',exact:true}).click();
  await page.locator('#restart-seed').click();
  await expect(page.locator('#current-preset')).toHaveText('Current run: Original.');
  expect((await snapshot(page,server)).experiment.protocol).toBe('wear-repair crowding v3');
});

test.describe('healthy repair-only row',()=> {
  test.use({serverArgs:['--mode','autonomous','--automata','repair:0,0,0,1/0,0,0,1/0,0,0,1/0,0,0,1','--width','3','--height','3','--occupancy','1','--upkeep','0','--crowding-upkeep','0','--ticks','2']});
  test('shows the conditional Wait fallback without inventing a repair threshold',async({page,server})=> {
    await page.goto(server.url); const initial=await snapshot(page,server);
    await inspectAgent(page,initial.cells.find(Boolean).id);
    await expect(page.locator('#agent-automaton svg')).toHaveAttribute('aria-label','Repair to Wait: 100%');
    await page.locator('#step').click(); await expect(page.locator('#tick')).toHaveText('1');
    await expect(page.locator('#inspection')).toContainText('Last selected action: Wait');
    expect((await snapshot(page,server)).totals.repairs).toBe('0');
  });
});
