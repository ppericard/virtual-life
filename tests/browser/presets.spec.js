import { test, expect, openPanel, inspectAgent } from './fixtures.js';

const runHeader = 'x-virtuallife-run';
async function read(page, server) {
  const reply = await page.request.get(`${server.url}/api/snapshot`);
  return {run: reply.headers()[runHeader], sample: await reply.json()};
}
async function save(page, info, name) {
  const path = info.outputPath(`${name}.png`);
  await page.screenshot({path, fullPage: true});
  await info.attach(name, {path, contentType: 'image/png'});
}

test.use({serverArgs: ['--mode', 'autonomous', '--width', '8', '--height', '6', '--ticks', '12', '--tick-ms', '60000', '--crowding-threshold', '3', '--crowding-upkeep', '2']});

test('preset choices stay pending until restart and synchronize both pages to the new run', async ({page, context, server}, info) => {
  await page.goto(server.url); await openPanel(page, 'New experiment');
  await expect(page.locator('#current-preset')).toHaveText('Current run: Mixed automata.');
  for (const name of ['Mixed automata', 'Movement runs', 'Copy bursts', 'Repair cycles', 'Wait cycles', 'Open-world trial', 'Roamers', 'Burst copiers', 'Settlers']) {
    await expect(page.getByRole('button', {name, exact: true})).toBeVisible();
  }
  await expect(page.locator('#preset-controls')).toContainText('Integrity and crowding adjust their transition probabilities.');
  await page.locator('#step').click(); await expect(page.locator('#tick')).toHaveText('1');
  const first = await read(page, server);
  const agent = first.sample.cells.find(Boolean).id;
  await inspectAgent(page, agent);
  const second = await context.newPage(); await second.goto(server.url); await openPanel(second, 'New experiment');
  await expect(second.locator('#tick')).toHaveText('1');
  await inspectAgent(second, agent);
  for (let tick = 2; tick <= 12; tick++) {
    await page.locator('#step').click(); await expect(page.locator('#tick')).toHaveText(String(tick));
  }
  await expect(second.locator('#tick')).toHaveText('12');
  const before = await read(page, server);
  expect(before.sample.failure_history.records.length).toBeGreaterThan(0);
  const history = await page.locator('#samples').textContent();
  const plot = await page.locator('#plot').getAttribute('aria-label');
  const legend = await page.locator('#legend').textContent();
  const drawing = await page.locator('#grid').evaluate(canvas => canvas.toDataURL());
  let writes = 0;
  page.on('request', request => { if (request.method() === 'POST') writes++; });
  await page.getByRole('button', {name: 'Movement runs', exact: true}).click();
  await second.getByRole('button', {name: 'Repair cycles', exact: true}).click();
  await expect(page.locator('#next-preset')).toContainText('Next restart: Movement runs.');
  await expect(page.getByRole('button', {name: 'Movement runs', exact: true})).toHaveAttribute('aria-pressed', 'true');
  await page.waitForResponse('**/api/snapshot');
  expect(await read(page, server)).toEqual(before);
  await expect(page.locator('#current-preset')).toHaveText('Current run: Mixed automata.');
  await expect(page.locator('#samples')).toHaveText(history);
  await expect(page.locator('#plot')).toHaveAttribute('aria-label', plot);
  await expect(page.locator('#legend')).toHaveText(legend);
  expect(await page.locator('#grid').evaluate((canvas, drawing) => canvas.toDataURL() === drawing, drawing)).toBe(true);
  expect(writes).toBe(0);
  await save(page, info, 'preset-pending-wide');
  await page.locator('#seed').fill('18446744073709551615');
  await context.setOffline(true); await expect(page.locator('#connection')).toBeVisible();
  await context.setOffline(false); await expect(page.locator('#connection')).toBeHidden();
  await expect(page.locator('#next-preset')).toContainText('Next restart: Movement runs.');
  await expect(page.locator('#seed')).toHaveValue('18446744073709551615');
  await page.locator('#restart-seed').click();
  for (const target of [page, second]) {
    await expect(target.locator('#tick')).toHaveText('0');
    await expect(target.getByRole('status')).toHaveText('paused');
    await expect(target.locator('#current-preset')).toHaveText('Current run: Movement runs.');
    await expect(target.locator('#next-preset')).toContainText('Next restart: Movement runs.');
    await expect(target.locator('#agent')).toHaveValue('');
    await expect(target.locator('#samples')).toContainText('Tick 0 · 1 sample');
    await expect(target.locator('#failure-summary')).toContainText('0 cumulative failures');
  }
  const after = await read(page, server);
  expect(after.run).not.toBe(before.run);
  expect(after.sample.experiment.seed).toBe('18446744073709551615');
  expect(after.sample.experiment.preset).toBe('movement-runs');
  expect(after.sample.experiment.groups.map(group => group.automaton)).toEqual(after.sample.experiment.presets.find(preset => preset.id === after.sample.experiment.preset).automata);
  expect(after.sample.experiment.maintenance).toEqual(before.sample.experiment.maintenance);
  expect(writes).toBe(1);
  await page.reload();
  await openPanel(page, 'New experiment');
  await expect(page.locator('#current-preset')).toHaveText('Current run: Movement runs.');
  await expect(page.getByRole('button', {name: 'Movement runs', exact: true})).toHaveAttribute('aria-pressed', 'true');
  await page.locator('#preset-controls summary').click();
  await expect(page.locator('#preset-summary')).toContainText('A: Inherited action graph (proportion 1)');
  await page.setViewportSize({width:390, height:844});
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await save(page, info, 'preset-applied-narrow');
});

test('open-world trial previews three graphs and restarts with their inherited settings', async ({page, server}, info) => {
  await page.goto(server.url); await openPanel(page, 'New experiment');
  const before = await read(page, server);
  await page.getByRole('button', {name:'Open-world trial', exact:true}).click();
  await expect(page.locator('#next-preset')).toContainText('Next restart: Open-world trial.');
  expect(await read(page, server)).toEqual(before);
  await page.locator('#preset-controls summary').click();
  for (const group of ['A', 'B', 'C']) {
    await expect(page.locator('#preset-summary')).toContainText(`${group}: Inherited action graph (proportion 1)`);
  }
  await page.locator('#restart-seed').click();
  await expect(page.locator('#current-preset')).toHaveText('Current run: Open-world trial.');
  const after = await read(page, server);
  const catalog = after.sample.experiment.presets.find(p => p.id === 'open-world');
  expect(after.sample.experiment.groups.map(g => g.automaton)).toEqual(catalog.automata);
  expect(after.sample.experiment.groups.map(g => g.automaton_name)).toEqual(['Roamers','Burst copiers','Settlers']);
  expect(after.sample.experiment.maintenance).toEqual(before.sample.experiment.maintenance);
  expect(after.sample.experiment.groups.map(g => g.proportion)).toEqual(['1','1','1']);
  await page.setViewportSize({width:390,height:844});
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await save(page, info, 'open-world-trial-narrow');
});

test.describe('custom preset recovery', () => {
  test.use({serverArgs: ['--mode', 'autonomous', '--width', '9', '--height', '7', '--occupancy', '0.4', '--automata', 'wait:1,2,3,4/1,2,3,4/1,2,3,4/1,2,3,4;wait:4,3,2,1/4,3,2,1/4,3,2,1/4,3,2,1', '--proportions', '3,2', '--ticks', '12', '--tick-ms', '60000', '--sample-every', '7', '--integrity', '17', '--upkeep', '2', '--crowding-threshold', '3', '--crowding-upkeep', '2', '--move-wear', '3', '--copy-wear', '4', '--repair', '6']});
  test('pending named selection can return to current custom settings and a random preset replays', async ({page, server}, info) => {
    await page.goto(server.url); await openPanel(page, 'New experiment');
    await expect(page.locator('#current-preset')).toHaveText('Current run: Custom settings.');
    const before = await read(page, server);
    expect(before.sample.experiment.preset).toBeNull();
    await page.getByRole('button', {name:'Mixed automata', exact:true}).click();
    await expect(page.locator('#next-preset')).toContainText('Next restart: Mixed automata.');
    await page.getByRole('button', {name:'Keep current custom settings', exact:true}).click();
    await expect(page.locator('#next-preset')).toContainText('Next restart: Custom settings.');
    await page.locator('#preset-controls summary').click();
    await expect(page.locator('#preset-summary')).toContainText('A: Inherited action graph (proportion 3)');
    await expect(page.locator('#preset-summary')).toContainText('B: Inherited action graph (proportion 2)');
    const customRequest = page.waitForRequest('**/api/restart');
    await page.locator('#restart-seed').click();
    expect((await customRequest).postDataJSON()).toEqual({seed:'1'});
    await expect(page.locator('#run-message')).toContainText('New experiment connected.');
    expect((await read(page, server)).sample).toEqual(before.sample);
    await expect(page.locator('#current-preset')).toHaveText('Current run: Custom settings.');
    await page.setViewportSize({width:390,height:844});
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await save(page, info, 'preset-custom-narrow');
    await page.getByRole('button', {name:'Copy bursts', exact:true}).click();
    const changed = page.waitForResponse('**/api/restart');
    await page.locator('#restart-random').click();
    const randomReply = await changed;
    expect(randomReply.request().postDataJSON()).toEqual({random:true,preset:'copy-bursts'});
    const random = await randomReply.json();
    await expect(page.locator('#current-preset')).toHaveText('Current run: Copy bursts.');
    await expect(page.locator('#seed')).toHaveValue(random.experiment.seed);
    expect([random.width,random.height,random.end_tick,random.experiment.occupancy]).toEqual([9,7,'12','0.400000']);
    expect(random.experiment.maintenance).toEqual(before.sample.experiment.maintenance);
    expect(random.experiment.groups.map(group => group.proportion)).toEqual(['1']);
    expect(random.experiment.groups.map(group => group.automaton)).toEqual(random.experiment.presets.find(preset => preset.id === random.experiment.preset).automata);
    await expect(page.locator('#preset-summary')).toContainText('A: Inherited action graph (proportion 1)');
    await save(page, info, 'preset-wide-movement-narrow');
    await expect(page.getByRole('button', {name:'Keep current custom settings', exact:true})).toHaveCount(0);
    await page.reload();
    await openPanel(page, 'New experiment');
    await expect(page.locator('#current-preset')).toHaveText('Current run: Copy bursts.');
    const replay = page.waitForResponse('**/api/restart');
    await page.locator('#restart-seed').click();
    const replayReply = await replay;
    expect(replayReply.request().postDataJSON()).toEqual({seed:random.experiment.seed});
    expect(await replayReply.json()).toEqual(random);
  });
});

test('preset acknowledgement locks choices and an older snapshot cannot undo a later pending edit', async ({page,server}) => {
  await page.goto(server.url); await openPanel(page, 'New experiment'); await expect(page.locator('#current-preset')).toHaveText('Current run: Mixed automata.');
  const oldRead = Promise.withResolvers(), releaseRead = Promise.withResolvers();
  const applied = Promise.withResolvers(), releaseReply = Promise.withResolvers();
  let reads = 0, writes = 0;
  await page.route('**/api/snapshot', async route => {
    if (++reads !== 1) return route.continue();
    const reply = await route.fetch(); oldRead.resolve();
    await releaseRead.promise; await route.fulfill({response:reply});
  });
  await page.route('**/api/restart', async route => {
    writes++; const reply = await route.fetch(); expect(reply.status()).toBe(200);
    applied.resolve(); await releaseReply.promise; await route.fulfill({response:reply});
  });
  try {
    await oldRead.promise;
    await page.getByRole('button', {name:'Movement runs',exact:true}).click();
    await page.locator('#restart-seed').click(); await applied.promise;
    for (const id of ['step','pause','resume','seed','restart-seed','restart-random']) await expect(page.locator(`#${id}`)).toBeDisabled();
    for (const button of await page.locator('#preset-options button').all()) await expect(button).toBeDisabled();
    const current = await read(page,server);
    expect(current.sample.experiment.preset).toBe('movement-runs');
    releaseReply.resolve();
    await expect(page.locator('#current-preset')).toHaveText('Current run: Movement runs.');
    await page.getByRole('button', {name:'Copy bursts',exact:true}).click();
    const fresh = page.waitForResponse(reply => reply.url().endsWith('/api/snapshot') && reply.headers()[runHeader] === current.run);
    releaseRead.resolve(); await fresh;
    await expect(page.locator('#next-preset')).toContainText('Next restart: Copy bursts.');
    await expect(page.locator('#current-preset')).toHaveText('Current run: Movement runs.');
    await expect(page.locator('#samples')).toContainText('Tick 0 · 1 sample');
    expect(await read(page,server)).toEqual(current);
    expect(writes).toBe(1);
  } finally { releaseRead.resolve(); releaseReply.resolve(); }
});

test('a lost preset reply is reconciled from the real run without retrying', async ({page,server}) => {
  await page.goto(server.url); await openPanel(page, 'New experiment'); await expect(page.locator('#current-preset')).toHaveText('Current run: Mixed automata.');
  const held = Promise.withResolvers(), release = Promise.withResolvers();
  let writes = 0;
  await page.route('**/api/snapshot', async route => { held.resolve(); await release.promise; await route.continue(); });
  await page.route('**/api/restart', async route => {
    writes++; const reply = await route.fetch(); expect(reply.status()).toBe(200); await route.abort('failed');
  });
  try {
    await held.promise;
    await page.getByRole('button', {name:'Repair cycles',exact:true}).click();
    await page.locator('#restart-seed').click();
    await expect(page.locator('#control-message')).toContainText('Restart outcome unknown');
    await expect(page.locator('#current-preset')).toHaveText('Current run: Mixed automata.');
    for (const button of await page.locator('#preset-options button').all()) await expect(button).toBeDisabled();
    const current = await read(page,server);
    expect(current.sample.experiment.preset).toBe('repair-cycles');
    release.resolve();
    await expect(page.locator('#current-preset')).toHaveText('Current run: Repair cycles.');
    await expect(page.locator('#next-preset')).toContainText('Next restart: Repair cycles.');
    await expect(page.locator('#restart-seed')).toBeEnabled();
    await page.waitForResponse('**/api/snapshot');
    expect(writes).toBe(1);
    expect(await read(page,server)).toEqual(current);
  } finally { release.resolve(); }
});

for (const kind of ['control','restart']) {
  test(`a late old ${kind} reply cannot roll back the current preset or pending choice`, async ({page,context,server}) => {
    await page.goto(server.url); await openPanel(page, 'New experiment'); await expect(page.locator('#current-preset')).toHaveText('Current run: Mixed automata.');
    const second = await context.newPage(); await second.goto(server.url); await openPanel(second, 'New experiment');
    await expect(second.locator('#current-preset')).toHaveText('Current run: Mixed automata.');
    const held = Promise.withResolvers(), release = Promise.withResolvers();
    let writes = 0;
    await page.route(`**/api/${kind}`, async route => {
      writes++; const reply = await route.fetch(); expect(reply.status()).toBe(200);
      held.resolve(); await release.promise; await route.fulfill({response:reply});
    });
    try {
      await page.getByRole('button', {name:'Movement runs',exact:true}).click();
      await page.locator(kind === 'control' ? '#step' : '#restart-seed').click();
      await held.promise;
      if (kind === 'restart') await expect(second.locator('#current-preset')).toHaveText('Current run: Movement runs.');
      await second.getByRole('button', {name:'Repair cycles',exact:true}).click();
      await second.locator('#restart-seed').click();
      await expect(page.locator('#current-preset')).toHaveText('Current run: Repair cycles.');
      await page.getByRole('button', {name:'Copy bursts',exact:true}).click();
      const late = page.waitForResponse(`**/api/${kind}`);
      release.resolve(); await late;
      await page.waitForResponse('**/api/snapshot');
      await expect(page.locator('#current-preset')).toHaveText('Current run: Repair cycles.');
      await expect(page.locator('#next-preset')).toContainText('Next restart: Copy bursts.');
      await expect(page.locator('#restart-seed')).toBeEnabled();
      await expect(page.locator('#samples')).toContainText('Tick 0 · 1 sample');
      expect((await read(page,server)).sample.experiment.preset).toBe('repair-cycles');
      expect(writes).toBe(1);
    } finally { release.resolve(); }
  });
}

test.describe('zero-tick preset run', () => {
  test.use({serverArgs:['--mode','autonomous','--ticks','0']});
  test('a preset applied to a zero-tick run stays completed', async ({page,server}, info) => {
    await page.goto(server.url); await openPanel(page, 'New experiment'); await expect(page.getByRole('status')).toHaveText('completed');
    await page.getByRole('button', {name:'Repair cycles',exact:true}).click();
    await page.locator('#restart-seed').click();
    await expect(page.locator('#current-preset')).toHaveText('Current run: Repair cycles.');
    await expect(page.getByRole('status')).toHaveText('completed');
    await expect(page.locator('#tick')).toHaveText('0');
    for (const id of ['step','pause','resume']) await expect(page.locator(`#${id}`)).toBeDisabled();
    const applied = (await read(page,server)).sample;
    expect(applied.experiment.preset).toBe('repair-cycles');
    expect(applied.experiment.groups.map(group => group.automaton)).toEqual(applied.experiment.presets.find(preset => preset.id === applied.experiment.preset).automata);
    await page.locator('#preset-controls summary').click();
    await expect(page.locator('#preset-summary')).toContainText('A: Inherited action graph (proportion 1)');
    await save(page, info, 'preset-repair-cycles-wide');
  });
});

test.describe('presets unavailable in demo', () => {
  test.use({serverArgs:['--mode','demo']});
  test('autonomous restart controls stay hidden', async ({page,server}) => {
    await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
    await expect(page.locator('#preset-controls')).toBeHidden();
    await expect(page.locator('#preset-options button')).toHaveCount(0);
    await expect(page.locator('#restart-controls')).toBeHidden();
  });
});
