import { test, expect } from './fixtures.js';

const snapshot = async (page, server) => (await page.request.get(`${server.url}/api/snapshot`)).json();

async function pageState(page) {
  return page.evaluate(() => {
    const byId = id => document.getElementById(id);
    return {
      tick: byId('tick').textContent, count: byId('count').textContent,
      status: byId('status').textContent, plot: byId('plot').getAttribute('aria-label'),
      selected: byId('agent').value, message: byId('control-message').textContent,
      warningVisible: byId('connection').getClientRects().length > 0,
      warning: byId('connection').textContent,
      stepDisabled: byId('step').disabled, pauseDisabled: byId('pause').disabled,
      resumeDisabled: byId('resume').disabled,
    };
  });
}

function freshRunReady(state) {
  return state.tick === '0' && state.count === '3' && state.status === 'paused'
    && state.plot === 'Tick 0: 3 agents' && state.selected === ''
    && !state.stepDisabled && !state.resumeDisabled && state.pauseDisabled
    && !state.message.includes('applied at tick');
}

function requiresReload(state) {
  return state.warningVisible && /reload/i.test(state.warning)
    && /run|experiment|restart|server/i.test(state.warning)
    && state.stepDisabled && state.resumeDisabled && state.pauseDisabled;
}

// Keep both recovery choices open: a coherent new run, or an explicit reload
// requirement with controls blocked. Silent mixing/stalling satisfies neither.
async function expectSafeRecovery(page) {
  await expect.poll(async () => {
    const state = await pageState(page);
    return freshRunReady(state) || requiresReload(state);
  }, { message: 'A new process must be separated from the old run or explicitly require reload' }).toBe(true);
}

async function holdSnapshots(page) {
  const requested = Promise.withResolvers(), release = Promise.withResolvers();
  await page.route('**/api/snapshot', async route => {
    requested.resolve();
    await release.promise;
    await route.continue();
  });
  return { requested: requested.promise, release: release.resolve };
}

async function evidence(page, server, info, controls) {
  await info.attach('restart-state.json', {
    body: JSON.stringify({ controls, page: await pageState(page), server: await snapshot(page, server) }, null, 2),
    contentType: 'application/json',
  });
}

test('reloading after a real server restart gives a clean new fixture', async ({ page, server }) => {
  await page.goto(server.url);
  await page.getByRole('button', { name: 'Single step' }).click();
  await expect(page.locator('#tick')).toHaveText('1');
  await server.restart();
  await page.reload();
  await expect.poll(async () => freshRunReady(await pageState(page))).toBe(true);
  expect(await snapshot(page, server)).toMatchObject({ tick: '0', status: 'paused', count: '3' });
  await page.getByRole('button', { name: 'Single step' }).click();
  await expect(page.locator('#tick')).toHaveText('1');
});

test('a retained page must not silently mix histories across a real server restart', async ({ page, server }, info) => {
  await page.goto(server.url);
  for (let tick = 1; tick <= 5; tick++) {
    await page.getByRole('button', { name: 'Single step' }).click();
    await expect(page.locator('#tick')).toHaveText(String(tick));
  }
  await expect(page.locator('#samples')).toContainText('Ticks 0–5 · 6 samples');
  await page.locator('#agent').selectOption('5');
  const gate = await holdSnapshots(page);
  try {
    await gate.requested; // No post-restart browser read can reach the old server.
    await server.restart();
    expect(await snapshot(page, server)).toMatchObject({ tick: '0', status: 'paused', count: '3' });
    gate.release();
    await expectSafeRecovery(page);
    await expectNewExperiment(page);
  } finally {
    gate.release();
    await page.unrouteAll({ behavior: 'wait' });
    await evidence(page, server, info);
  }
});

test('an old Step receipt must not silently strand a retained page after a real restart', async ({ page, server }, info) => {
  await page.goto(server.url);
  await expect(page.getByRole('button', { name: 'Single step' })).toBeEnabled();
  const controls = [];
  page.on('request', request => {
    if (request.url().endsWith('/api/control')) controls.push(request.postData());
  });
  const gate = await holdSnapshots(page);
  try {
    await gate.requested; // Prevent the tick-1 snapshot from satisfying the receipt.
    await page.getByRole('button', { name: 'Single step' }).click();
    await expect(page.locator('#control-message')).toHaveText('Step applied at tick 1.');
    await expect.poll(async () => (await snapshot(page, server)).tick).toBe('1');
    await server.restart();
    expect(await snapshot(page, server)).toMatchObject({ tick: '0', status: 'paused', count: '3' });
    gate.release();
    await expectSafeRecovery(page);
    await expectNewExperiment(page);
    expect(controls).toEqual(['{"command":"step"}']);
    expect(await snapshot(page, server)).toMatchObject({ tick: '0', status: 'paused' });
  } finally {
    gate.release();
    await page.unrouteAll({ behavior: 'wait' });
    await evidence(page, server, info, controls);
  }
});

// The approved policy is automatic recovery, not the alternative reload prompt.
async function expectNewExperiment(page, tick = '0', status = 'paused') {
  await expect(page.locator('#run-message')).toContainText('New experiment connected.');
  await expect(page.locator('#tick')).toHaveText(tick);
  await expect(page.getByRole('status')).toHaveText(status);
  await expect(page.locator('#agent')).toHaveValue('');
  await expect(page.locator('#control-message')).toHaveText('');
  await expect(page.locator('#connection')).toBeHidden();
}

const runHeader = 'x-virtuallife-run';
const runId = async (page, server) => (await page.request.get(`${server.url}/api/snapshot`)).headers()[runHeader];

test.describe('approved automatic recovery', () => {
  test.use({ serverArgs: ['--tick-ms', '60000'] });

  test('same-tick restart clears selection and follows an already running new experiment', async ({ page, server }, info) => {
    await page.goto(server.url);
    await expect(page.locator('#tick')).toHaveText('0');
    await page.locator('#agent').selectOption('2');
    const oldId = await runId(page, server);
    expect(oldId).toMatch(/^[0-9a-f]{32}$/);
    const gate = await holdSnapshots(page);
    const controls = [];
    page.on('request', request => { if (request.url().endsWith('/api/control')) controls.push(request.postData()); });
    try {
      await gate.requested;
      await server.restart();
      const newId = await runId(page, server);
      expect(newId).toMatch(/^[0-9a-f]{32}$/);
      expect(newId).not.toBe(oldId);
      // An independent API client starts the new run before the page sees it.
      // The page must follow that state, not force a pause/resume of its own.
      const reply = await page.request.post(`${server.url}/api/control`, {
        headers: { Origin: server.url, [runHeader]: newId }, data: { command: 'resume' },
      });
      expect(reply.status()).toBe(200);
      expect(reply.headers()[runHeader]).toBe(newId);
      gate.release();
      await expectNewExperiment(page, '0', 'running');
      await expect(page.locator('#plot')).toHaveAttribute('aria-label', 'Tick 0: 3 agents');
      await expect(page.getByRole('button', { name: 'Pause', exact: true })).toBeEnabled();
      await expect(page.getByRole('button', { name: 'Single step' })).toBeDisabled();
      expect(controls).toEqual([]);
      const file = info.outputPath('new-experiment.png');
      await page.screenshot({ path: file, fullPage: true });
      await info.attach('new-experiment', { path: file, contentType: 'image/png' });
    } finally {
      gate.release();
      await page.unrouteAll({ behavior: 'wait' });
    }
  });

  test('same-run reconnect retains history and selection without a new-run notice', async ({ page, context, server }) => {
    await page.goto(server.url);
    await page.getByRole('button', { name: 'Single step' }).click();
    await expect(page.locator('#tick')).toHaveText('1');
    await page.locator('#agent').selectOption('2');
    const identity = await runId(page, server);
    const history = await page.locator('#plot').getAttribute('aria-label');
    await context.setOffline(true);
    await expect(page.locator('#connection')).toBeVisible();
    await context.setOffline(false);
    await expect(page.locator('#connection')).toBeHidden();
    expect(await runId(page, server)).toBe(identity);
    await expect(page.locator('#plot')).toHaveAttribute('aria-label', history);
    await expect(page.locator('#agent')).toHaveValue('2');
    await expect(page.locator('#run-message')).toHaveText('');
    await expect(page.getByRole('button', { name: 'Single step' })).toBeEnabled();
  });

  test('an old command reply cannot overwrite controls or messages in the new run', async ({ page, server }) => {
    await page.goto(server.url);
    await expect(page.getByRole('button', { name: 'Single step' })).toBeEnabled();
    const oldId = await runId(page, server);
    const held = Promise.withResolvers(), release = Promise.withResolvers(), delivered = Promise.withResolvers();
    let requests = 0, oldRequestFailed = false;
    page.on('requestfailed', request => {
      if (request.url().endsWith('/api/control') && request.headers()[runHeader] === oldId) oldRequestFailed = true;
    });
    await page.route('**/api/control', async route => {
      if (++requests !== 1) return route.continue();
      const response = await route.fetch();
      held.resolve({ status: response.status(), body: await response.json(), id: response.headers()[runHeader] });
      await release.promise;
      await route.fulfill({ response });
      delivered.resolve();
    });
    try {
      await page.getByRole('button', { name: 'Single step' }).click();
      expect(await held.promise).toMatchObject({ status: 200, body: { tick: '1' }, id: oldId });
      await server.restart();
      await expectNewExperiment(page);
      for (let tick = 1; tick <= 2; tick++) {
        await page.getByRole('button', { name: 'Single step' }).click();
        await expect(page.locator('#tick')).toHaveText(String(tick));
      }
      // A timeout would fail this test rather than masquerade as a late reply.
      expect(oldRequestFailed).toBe(false);
      release.resolve();
      await delivered.promise;
      await page.waitForResponse('**/api/snapshot');
      await expect(page.locator('#control-message')).toHaveText('Step applied at tick 2.');
      await expect(page.locator('#tick')).toHaveText('2');
      await expect(page.getByRole('button', { name: 'Single step' })).toBeEnabled();
      await expect(page.locator('#connection')).toBeHidden();
      expect(requests).toBe(3);
    } finally {
      release.resolve();
      await page.unrouteAll({ behavior: 'wait' });
    }
  });

  test('a delayed old-run command is rejected before it can step the new server', async ({ page, server }) => {
    await page.goto(server.url);
    await expect(page.getByRole('button', { name: 'Single step' })).toBeEnabled();
    const oldId = await runId(page, server);
    const held = Promise.withResolvers(), release = Promise.withResolvers();
    let requests = 0;
    await page.route('**/api/control', async route => {
      requests++;
      held.resolve(route.request().headers()[runHeader]);
      await release.promise;
      await route.continue();
    });
    try {
      await page.getByRole('button', { name: 'Single step' }).click();
      expect(await held.promise).toBe(oldId);
      await server.restart();
      await expectNewExperiment(page);
      const response = page.waitForResponse('**/api/control');
      release.resolve();
      const rejected = await response;
      expect(rejected.status()).toBe(409);
      expect(await rejected.headerValue(runHeader)).not.toBe(oldId);
      expect(await rejected.json()).toMatchObject({ error: 'experiment changed; read the new snapshot' });
      await page.waitForResponse('**/api/snapshot');
      expect(await snapshot(page, server)).toMatchObject({ tick: '0', status: 'paused', count: '3' });
      await expect(page.locator('#control-message')).toHaveText('');
      await expect(page.getByRole('button', { name: 'Single step' })).toBeEnabled();
      expect(requests).toBe(1);
    } finally {
      release.resolve();
      await page.unrouteAll({ behavior: 'wait' });
    }
  });
});
