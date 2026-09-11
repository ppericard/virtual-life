import { test, expect, openPanel, inspectAgent } from './fixtures.js';

const runHeader = 'x-virtuallife-run';
async function read(page, server) {
  const reply = await page.request.get(`${server.url}/api/snapshot`);
  return { run: reply.headers()[runHeader], sample: await reply.json() };
}
async function restart(page, server, seed) {
  const { run } = await read(page, server);
  const reply = await page.request.post(`${server.url}/api/restart`, {
    headers: { Origin: server.url, [runHeader]: run }, data: { seed },
  });
  expect(reply.status()).toBe(200);
  return { run: reply.headers()[runHeader], sample: await reply.json() };
}
async function ready(page, seed = '1') {
  await expect(page.locator('#tick')).toHaveText('0');
  await expect(page.getByRole('status')).toHaveText('paused');
  await openPanel(page, 'New experiment');
  await expect(page.getByLabel('Seed', { exact: true })).toHaveValue(seed);
  await expect(page.locator('#restart-seed')).toBeEnabled();
}
async function step(page, tick) {
  await page.locator('#step').click();
  await expect(page.locator('#tick')).toHaveText(String(tick));
}
async function mutationsLocked(page) {
  for (const id of ['step', 'pause', 'resume', 'restart-seed', 'restart-random']) await expect(page.locator(`#${id}`)).toBeDisabled();
}

test.describe('seeded web restarts', () => {
  test.use({ serverArgs: ['--mode', 'autonomous', '--width', '8', '--height', '6', '--ticks', '12', '--tick-ms', '60000', '--crowding-threshold', '3', '--crowding-upkeep', '2'] });

  test('fixed seed replays initial and final worlds, clears both pages and wraps on narrow screens', async ({ page, context, server }, info) => {
    await page.goto(server.url); await ready(page);
    const initial = await read(page, server);
    expect(initial.sample.experiment.maintenance).toMatchObject({crowding_threshold:3,crowding_upkeep:2});
    const second = await context.newPage(); await second.goto(server.url); await ready(second);
    const selected = initial.sample.cells.find(Boolean).id;
    await inspectAgent(page, selected);
    await inspectAgent(second, selected);
    for (let tick = 1; tick <= 12; tick++) await step(page, tick);
    await expect(second.locator('#tick')).toHaveText('12');
    const final = (await read(page, server)).sample;
    expect(final.failure_history.records.length).toBeGreaterThan(0);
    await expect(page.locator('#restart-seed')).toBeEnabled();
    await page.locator('#restart-seed').click();
    await ready(page); await ready(second);
    const replaced = await read(page, server);
    expect(replaced.run).not.toBe(initial.run);
    expect(replaced.sample).toEqual(initial.sample);
    for (const target of [page, second]) {
      await expect(target.locator('#samples')).toContainText('Tick 0 · 1 sample');
      await expect(target.locator('#agent')).toHaveValue('');
      await expect(target.locator('#failure-summary')).toContainText('0 cumulative failures');
      await expect(target.locator('#run-message')).toContainText('New experiment connected.');
    }
    for (let tick = 1; tick <= 12; tick++) await step(page, tick);
    expect((await read(page, server)).sample).toEqual(final);
    await page.setViewportSize({ width: 390, height: 950 });
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const screenshot = info.outputPath('seeded-restart-narrow.png');
    await page.screenshot({ path: screenshot, fullPage: true });
    await info.attach('seeded-restart-narrow', { path: screenshot, contentType: 'image/png' });
  });

  test('seed edits survive polling, invalid values send nothing and OS-random seed is replayable', async ({ page, context, server }) => {
    await page.goto(server.url); await ready(page);
    let requests = 0;
    page.on('request', request => { if (request.url().endsWith('/api/restart')) requests++; });
    for (const seed of ['', '-1', '+1', '1.0', '1e2', ' 1', '18446744073709551616']) {
      await page.getByLabel('Seed', { exact: true }).fill(seed);
      await page.locator('#restart-seed').click();
      await expect(page.getByLabel('Seed', { exact: true })).toHaveAttribute('aria-invalid', 'true');
      await expect(page.locator('#control-message')).toContainText('using digits only');
    }
    expect(requests).toBe(0);
    const max = '18446744073709551615';
    await page.getByLabel('Seed', { exact: true }).fill(max);
    await page.waitForResponse('**/api/snapshot');
    await expect(page.getByLabel('Seed', { exact: true })).toHaveValue(max);
    await context.setOffline(true); await expect(page.locator('#connection')).toBeVisible();
    await context.setOffline(false); await expect(page.locator('#connection')).toBeHidden();
    await expect(page.getByLabel('Seed', { exact: true })).toHaveValue(max);
    await page.locator('#restart-seed').click(); await ready(page, max);
    expect((await read(page, server)).sample.experiment.seed).toBe(max);
    await page.locator('#resume').click(); await expect(page.getByRole('status')).toHaveText('running');
    const changed = page.waitForResponse('**/api/restart');
    await page.locator('#restart-random').click();
    const randomResponse = await changed;
    expect(randomResponse.status()).toBe(200);
    const random = await randomResponse.json();
    expect(random.experiment.maintenance).toMatchObject({crowding_threshold:3,crowding_upkeep:2});
    const seed = random.experiment.seed;
    expect(seed).toMatch(/^\d+$/); expect(BigInt(seed)).toBeLessThanOrEqual(18446744073709551615n);
    await ready(page, seed);
    await page.locator('#restart-seed').click(); await ready(page, seed);
    expect((await read(page, server)).sample).toEqual(random);
    expect(requests).toBe(3);
  });

  test('a lost restart reply has unknown outcome and is never automatically retried', async ({ page, server }) => {
    await page.goto(server.url); await ready(page); await step(page, 1);
    const hold = Promise.withResolvers(), release = Promise.withResolvers();
    await page.route('**/api/snapshot', async route => { hold.resolve(); await release.promise; await route.continue(); });
    let requests = 0;
    await page.route('**/api/restart', async route => {
      requests++; const response = await route.fetch(); expect(response.status()).toBe(200);
      await route.abort('failed');
    });
    try {
      await hold.promise;
      await page.locator('#restart-random').click();
      await expect(page.locator('#control-message')).toContainText('Restart outcome unknown');
      await mutationsLocked(page);
      const actual = await read(page, server);
      expect(actual.sample.tick).toBe('0');
      release.resolve(); await ready(page, actual.sample.experiment.seed);
      await page.waitForResponse('**/api/snapshot');
      expect(requests).toBe(1);
      expect((await read(page, server)).run).toBe(actual.run);
      await expect(page.locator('#samples')).toContainText('Tick 0 · 1 sample');
    } finally { release.resolve(); await page.unrouteAll({ behavior: 'wait' }); }
  });

  test('acknowledged restart retires a delayed old snapshot and locks every mutation until acknowledgement', async ({ page, server }) => {
    await page.goto(server.url); await ready(page); await step(page, 1);
    const snapshotHeld = Promise.withResolvers(), releaseSnapshot = Promise.withResolvers();
    const snapshotDelivered = Promise.withResolvers();
    let reads = 0;
    await page.route('**/api/snapshot', async route => {
      if (++reads !== 1) return route.continue();
      const response = await route.fetch(); snapshotHeld.resolve(await response.json());
      await releaseSnapshot.promise; await route.fulfill({ response }); snapshotDelivered.resolve();
    });
    const restartHeld = Promise.withResolvers(), releaseRestart = Promise.withResolvers();
    await page.route('**/api/restart', async route => {
      const response = await route.fetch(); restartHeld.resolve(); await releaseRestart.promise; await route.fulfill({ response });
    });
    try {
      expect((await snapshotHeld.promise).tick).toBe('1');
      await page.locator('#restart-seed').click(); await restartHeld.promise;
      await mutationsLocked(page);
      releaseRestart.resolve(); await ready(page);
      releaseSnapshot.resolve(); await snapshotDelivered.promise;
      await page.waitForResponse('**/api/snapshot'); await ready(page);
      await expect(page.locator('#samples')).toContainText('Tick 0 · 1 sample');
      expect((await read(page, server)).sample.tick).toBe('0');
    } finally { releaseRestart.resolve(); releaseSnapshot.resolve(); await page.unrouteAll({ behavior: 'wait' }); }
  });

  for (const path of ['control', 'restart']) {
    test(`a delayed old ${path} request is rejected by the replacement run`, async ({ page, server }) => {
      await page.goto(server.url); await ready(page);
      const old = await read(page, server);
      const held = Promise.withResolvers(), release = Promise.withResolvers();
      let requests = 0;
      await page.route(`**/api/${path}`, async route => {
        requests++; held.resolve(route.request().headers()[runHeader]);
        await release.promise; await route.continue();
      });
      try {
        await page.locator(path === 'control' ? '#step' : '#restart-seed').click();
        expect(await held.promise).toBe(old.run);
        await mutationsLocked(page);
        const next = await restart(page, server, '42');
        await ready(page, '42');
        const rejected = page.waitForResponse(`**/api/${path}`);
        release.resolve(); const response = await rejected;
        expect(response.status()).toBe(409);
        expect(await response.headerValue(runHeader)).toBe(next.run);
        await page.waitForResponse('**/api/snapshot');
        expect(await read(page, server)).toEqual(next);
        await expect(page.locator('#control-message')).toHaveText('');
        expect(requests).toBe(1);
      } finally { release.resolve(); await page.unrouteAll({ behavior: 'wait' }); }
    });

    test(`a late old ${path} reply cannot roll back a newer restart`, async ({ page, server }) => {
      await page.goto(server.url); await ready(page);
      const held = Promise.withResolvers(), release = Promise.withResolvers(), delivered = Promise.withResolvers();
      let requests = 0;
      let requestFailed = false;
      page.on('requestfailed', request => { if (request.url().endsWith(`/api/${path}`)) requestFailed = true; });
      await page.route(`**/api/${path}`, async route => {
        if (++requests !== 1) return route.continue();
        const response = await route.fetch(); held.resolve(response.status());
        await release.promise; await route.fulfill({ response }); delivered.resolve();
      });
      try {
        await page.locator(path === 'control' ? '#step' : '#restart-seed').click();
        expect(await held.promise).toBe(200);
        const next = await restart(page, server, '42');
        await ready(page, '42');
        await step(page, 1);
        expect(requestFailed).toBe(false); // A timeout cannot masquerade as a delivered late reply.
        release.resolve(); await delivered.promise;
        await page.waitForResponse('**/api/snapshot');
        await expect(page.locator('#tick')).toHaveText('1');
        await expect(page.locator('#seed')).toHaveValue('42');
        await expect(page.locator('#control-message')).toHaveText('Step applied at tick 1.');
        await expect(page.locator('#connection')).toBeHidden();
        expect((await read(page, server)).run).toBe(next.run);
      } finally { release.resolve(); await page.unrouteAll({ behavior: 'wait' }); }
    });
  }
});

test.describe('launched effective seed and zero tick lifecycle', () => {
  test.use({ serverArgs: ['--mode', 'autonomous', '--seed', '18446744073709551615', '--ticks', '0'] });
  test('launched seed remains exact and a zero tick run restarts completed', async ({ page, server }) => {
    await page.goto(server.url);
    await openPanel(page, 'New experiment');
    await expect(page.locator('#seed')).toHaveValue('18446744073709551615');
    await expect(page.getByRole('status')).toHaveText('completed');
    const old = await read(page, server);
    await page.locator('#seed').fill('0'); await page.locator('#restart-seed').click();
    await expect(page.locator('#configuration')).toContainText('Seed 0 ·');
    await expect(page.getByRole('status')).toHaveText('completed');
    await expect(page.locator('#restart-seed')).toBeEnabled();
    await expect(page.locator('#step')).toBeDisabled();
    const next = await read(page, server); expect(next.run).not.toBe(old.run);
    expect(next.sample).toMatchObject({ tick: '0', end_tick: '0', experiment: { seed: '0' } });
  });
});

test('the scripted demo retains its controls and hides seeded restart', async ({ page, server }) => {
  await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
  await expect(page.locator('#restart-controls')).toBeHidden();
  await step(page, 1);
});
