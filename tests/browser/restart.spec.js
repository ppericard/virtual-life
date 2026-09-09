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
  await expect(page.locator('#samples')).toContainText('Observed ticks: 0, 1, 2, 3, 4, 5.');
  await page.locator('#agent').selectOption('5');
  const gate = await holdSnapshots(page);
  try {
    await gate.requested; // No post-restart browser read can reach the old server.
    await server.restart();
    expect(await snapshot(page, server)).toMatchObject({ tick: '0', status: 'paused', count: '3' });
    gate.release();
    await expectSafeRecovery(page);
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
    expect(controls).toEqual(['{"command":"step"}']);
    expect(await snapshot(page, server)).toMatchObject({ tick: '0', status: 'paused' });
  } finally {
    gate.release();
    await page.unrouteAll({ behavior: 'wait' });
    await evidence(page, server, info, controls);
  }
});
