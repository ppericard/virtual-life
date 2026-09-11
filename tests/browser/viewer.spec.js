import { test, expect, openPanel } from './fixtures.js';

const states = [
  [[1,10,0,2],[2,20,2,2],[3,30,4,2]],
  [[1,10,0,2],[2,20,2,2],[3,31,4,2]],
  [[2,20,2,2],[4,20,3,2],[3,31,4,2]],
  [[3,31,0,2],[2,21,2,2],[4,20,3,3]],
  [[3,31,0,2],[5,21,1,2],[2,21,2,2],[4,20,3,3]],
  [[5,21,1,2],[2,21,2,2]],
];
const totals = [[0,0,0,0],[0,0,0,1],[0,1,1,1],[2,1,1,2],[2,2,1,2],[2,2,3,2]];
const colors = ['#c89bc4', '#89baca', '#d6b36e', '#b4bf83', '#97bba0'];
async function clickCell(page, x, y) {
  const canvas = page.locator('#grid'); const box = await canvas.boundingBox();
  await canvas.click({ position: { x: (8 + (x + .5) * 108) * box.width / 556, y: (8 + (y + .5) * 108) * box.height / 556 } });
}
async function checkState(page, server, tick) {
  await expect(page.locator('#tick')).toHaveText(String(tick));
  await expect(page.locator('#count')).toHaveText(String(states[tick].length));
  for (const [index, name] of ['moves','creations','removals','value_changes'].entries()) await expect(page.locator(`#${name}`)).toHaveText(String(totals[tick][index]));
  // Check actual canvas pixels at every square, away from ID labels/selection borders.
  const pixels = await page.locator('#grid').evaluate(canvas => {
    const ctx = canvas.getContext('2d');
    return Array.from({length: 25}, (_, i) => {
      const x = (8 + i % 5 * 108 + 18) * canvas.width / 556;
      const y = (8 + Math.floor(i / 5) * 108 + 18) * canvas.height / 556;
      const pixel = ctx.getImageData(x, y, 1, 1).data;
      return '#' + [...pixel].slice(0, 3).map(n => n.toString(16).padStart(2,'0')).join('');
    });
  });
  const expected = Array(25).fill('#f5f7f2');
  for (const [id,,x,y] of states[tick]) expected[y*5+x] = colors[id % colors.length];
  expect(pixels).toEqual(expected);
  const snapshot = await (await page.request.get(`${server.url}/api/snapshot`)).json();
  const actual = snapshot.cells.flatMap((agent, i) => agent ? [[agent.id, agent.value, i % 5, Math.floor(i / 5)]] : []);
  expect(actual).toEqual(states[tick].map(([id,value,x,y]) => [String(id),String(value),x,y]));
}
async function screenshot(page, info, name) {
  const file = info.outputPath(`${name}.png`);
  await page.screenshot({ path: file, fullPage: true });
  await info.attach(name, { path: file, contentType: 'image/png' });
}

test('initial pause and every MODEL transition, grid inspection, exact final state', async ({ page, server }, info) => {
  await page.goto(server.url);
  await expect(page.getByRole('status')).toHaveText('paused');
  await checkState(page, server, 0);
  await expect(page.getByRole('button', {name:'Pause', exact:true})).toBeDisabled();
  await clickCell(page, 4, 2);
  await expect(page.locator('#inspection')).toHaveText('ID 3 · Value 30 · Position (4, 2)');
  await screenshot(page, info, 'paused');
  for (let tick = 1; tick <= 5; tick++) {
    await page.getByRole('button', {name:'Single step'}).click();
    await checkState(page, server, tick);
    const inspection = tick < 3 ? 'ID 3 · Value 31 · Position (4, 2)' : tick < 5 ? 'ID 3 · Value 31 · Position (0, 2)' : 'Agent 3 was removed by tick 5.';
    await expect(page.locator('#inspection')).toHaveText(inspection);
  }
  await expect(page.getByRole('status')).toHaveText('completed');
  for (const name of ['Single step','Resume','Pause']) await expect(page.getByRole('button', {name, exact:true})).toBeDisabled();
  await clickCell(page, 1, 2);
  await expect(page.locator('#inspection')).toHaveText('ID 5 · Value 21 · Position (1, 2)');
  await expect(page.locator('#samples')).toContainText('Ticks 0–5 · 6 samples · 0 sampling gaps.');
  await screenshot(page, info, 'completed');
  await page.reload();
  await checkState(page, server, 5);
  await expect(page.locator('#samples')).toContainText('Tick 5 · 1 sample');
});

test.describe('pacing and gaps', () => {
  test.use({serverArgs:['--mode','demo','--tick-ms','600','--sample-every','2']});
  test('pause/resume and sampled plot gaps in the real run', async ({page,server},info) => {
    await page.goto(server.url);
    await expect(page.locator('#tick')).toHaveText('0');
    await page.getByRole('button', {name:'Resume',exact:true}).click();
    await expect(page.getByRole('status')).toHaveText('running');
    await page.getByRole('button', {name:'Pause',exact:true}).click();
    await expect(page.getByRole('status')).toHaveText('paused');
    const paused = await page.locator('#tick').textContent();
    // A rejected step while running is covered by the API tests; UI prevents it.
    await page.getByRole('button', {name:'Resume',exact:true}).click();
    await expect(page.getByRole('button', {name:'Single step'})).toBeDisabled();
    await expect(page.getByRole('status')).toHaveText('completed');
    await checkState(page,server,5);
    await expect(page.locator('#samples')).toContainText('sampling gap');
    await openPanel(page, 'Analysis');
    const ticks = (await page.locator('#plot').getAttribute('aria-label')).match(/Tick (\d+)/g).map(s => Number(s.slice(5)));
    expect(ticks.some((tick,i) => i > 0 && tick-ticks[i-1]>1)).toBe(true);
    expect(ticks).toContain(Number(paused));
    // No invented line across the initial gap: center of 0→2 at count 3 is blank.
    if (ticks[0]===0 && ticks[1]===2) {
      const pixel = await page.locator('#plot').evaluate(canvas => [...canvas.getContext('2d').getImageData(148*canvas.width/600,39*canvas.height/230,1,1).data]);
      expect(pixel[3]).toBe(0);
    }
    await screenshot(page,info,'sampled-gaps');
  });
});

test.describe('completion without a page', () => {
  test.use({serverArgs:['--mode','demo','--running','--tick-ms','0']});
  test('a new page reconnects to the completed experiment', async ({page,server}) => {
    await expect.poll(async () => (await (await page.request.get(`${server.url}/api/snapshot`)).json()).status).toBe('completed');
    await page.goto(server.url);
    await checkState(page,server,5);
    await expect(page.locator('#samples')).toContainText('Tick 5 · 1 sample');
  });
});

test.describe('same-tick control reconciliation', () => {
  test.use({serverArgs:['--mode','demo','--tick-ms','60000']});
  test('a superseded receipt releases controls while pre-control reads stay ignored', async ({page,server}) => {
    await page.goto(server.url);
    await expect(page.getByRole('button', {name:'Resume',exact:true})).toBeEnabled();
    const staleRead = Promise.withResolvers(), releaseStale = Promise.withResolvers();
    const freshRead = Promise.withResolvers(), releaseFresh = Promise.withResolvers();
    let reads = 0;
    await page.route('**/api/snapshot', async route => {
      if (++reads === 1) {
        // Retain a real pre-control response; all later reads wait before reaching Rust.
        const response = await route.fetch();
        staleRead.resolve(await response.json());
        await releaseStale.promise;
        await route.fulfill({response});
      } else {
        freshRead.resolve();
        await releaseFresh.promise;
        await route.continue();
      }
    });
    const current = async () => {
      const snapshot = await (await page.request.get(`${server.url}/api/snapshot`)).json();
      return {tick:snapshot.tick,status:snapshot.status};
    };
    try {
      expect(await staleRead.promise).toMatchObject({tick:'0',status:'paused'});
      await page.getByRole('button', {name:'Resume',exact:true}).click();
      await expect(page.locator('#control-message')).toHaveText('Resume applied at tick 0.');
      await expect.poll(current).toEqual({tick:'0',status:'running'});
      // Another actual same-origin control supersedes Resume without advancing a tick.
      const paused = await page.request.post(`${server.url}/api/control`, {headers:{Origin:server.url},data:{command:'pause'}});
      expect(paused.status()).toBe(200);
      expect(await paused.json()).toMatchObject({applied:true,tick:'0',status:'paused'});
      await expect.poll(current).toEqual({tick:'0',status:'paused'});

      releaseStale.resolve();
      await freshRead.promise; // The single polling loop has consumed the stale reply.
      for (const name of ['Resume','Pause','Single step']) {
        await expect(page.getByRole('button', {name,exact:true})).toBeDisabled();
      }
      releaseFresh.resolve();
      await expect(page.getByRole('button', {name:'Resume',exact:true})).toBeEnabled();
      await expect(page.getByRole('button', {name:'Single step'})).toBeEnabled();
      await expect(page.getByRole('status')).toHaveText('paused');
      await expect(page.locator('#connection')).toBeHidden();
      await expect(page.locator('#tick')).toHaveText('0');
      await page.getByRole('button', {name:'Single step'}).click();
      await expect(page.locator('#tick')).toHaveText('1');
      await expect.poll(current).toEqual({tick:'1',status:'paused'});
    } finally {
      releaseStale.resolve();
      releaseFresh.resolve();
    }
  });
});

test('refresh and closing the only page never reset or stop the process', async ({page,context,server}) => {
  await page.goto(server.url);
  await page.getByRole('button', {name:'Single step'}).click();
  await expect(page.locator('#tick')).toHaveText('1');
  await page.reload();
  await expect(page.locator('#tick')).toHaveText('1');
  await expect(page.locator('#samples')).toContainText('Tick 1 · 1 sample');
  await page.getByRole('button', {name:'Resume',exact:true}).click();
  await expect(page.getByRole('status')).toHaveText('running');
  await page.close();
  // Only API observation remains; no page is connected while computation finishes.
  await expect.poll(async () => (await (await context.request.get(`${server.url}/api/snapshot`)).json()).status).toBe('completed');
  const reopened = await context.newPage(); await reopened.goto(server.url);
  await checkState(reopened,server,5);
});

test('completion/control race cannot successfully step beyond five', async ({page,server}) => {
  await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
  const post = command => page.request.post(`${server.url}/api/control`, {headers:{Origin:server.url},data:{command}});
  for(let i=0;i<4;i++) expect((await post('step')).status()).toBe(200);
  const responses = await Promise.all([post('step'),post('step')]);
  const replies = await Promise.all(responses.map(async response => ({code:response.status(),body:await response.json()})));
  expect(replies.filter(reply => reply.code===200)).toHaveLength(1);
  expect(replies.find(reply => reply.code===200).body.tick).toBe('5');
  await expect(page.getByRole('status')).toHaveText('completed');
  await checkState(page,server,5);
  expect((await post('resume')).status()).toBe(409);
  await expect(page.locator('#connection')).toBeHidden();
});

test('connection errors are visible, reconnect works, lost step reply is never retried', async ({page,context,server}) => {
  await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
  await context.setOffline(true);
  await expect(page.getByRole('alert')).toContainText('Connection lost');
  await expect(page.getByRole('button',{name:'Single step'})).toBeDisabled();
  await context.setOffline(false);
  await expect(page.getByRole('alert')).toBeHidden();
  let requests=0;
  // Deliver the real command, then lose its real response; never fake a simulation state.
  await page.route('**/api/control',async route => { requests++; await route.fetch(); await route.abort('failed'); });
  await page.getByRole('button',{name:'Single step'}).click();
  await expect(page.locator('#control-message')).toContainText('Outcome unknown');
  await expect(page.locator('#tick')).toHaveText('1');
  expect(requests).toBe(1);
  await server.stop();
  await expect(page.getByRole('alert')).toContainText('Connection lost');
  await expect(page.getByRole('button',{name:'Single step'})).toBeDisabled();
});

test('narrow window remains usable and exact integer readouts do not round', async ({page,server},info) => {
  await page.setViewportSize({width:390,height:844});
  await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
  await clickCell(page,2,2);
  await expect(page.locator('#inspection')).toHaveText('ID 2 · Value 20 · Position (2, 2)');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  await screenshot(page,info,'narrow-paused');
  // Supplemental display boundary check, separate from all real-fixture E2E scenarios.
  await page.route('**/api/snapshot',route => route.abort());
  await page.evaluate(async () => {
    const { renderReadouts } = await import('/display.js');
    renderReadouts({tick:'18446744073709551615',count:'9007199254740993',status:'completed',width:3,cells:[{id:'18446744073709551614',value:'-9223372036854775808'}],totals:{moves:'18446744073709551615',creations:'9007199254740993',removals:'0',value_changes:'18446744073709551614'}},'18446744073709551614');
  });
  await expect(page.locator('#tick')).toHaveText('18446744073709551615');
  await expect(page.locator('#count')).toHaveText('9007199254740993');
  await expect(page.locator('#moves')).toHaveText('18446744073709551615');
  await expect(page.locator('#inspection')).toHaveText('ID 18446744073709551614 · Value -9223372036854775808 · Position (0, 0)');
  await page.evaluate(async () => {
    const { renderReadouts }=await import('/display.js');
    renderReadouts({tick:'9007199254740993',count:'1',status:'paused',width:3,cells:[{id:'9007199254740993',value:'9223372036854775807'}],totals:{}},'9007199254740993');
  });
  await expect(page.locator('#inspection')).toContainText('Value 9223372036854775807');
});
