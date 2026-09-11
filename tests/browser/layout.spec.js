import { test, expect, waitForGridFit } from './fixtures.js';

test.use({
  viewport: { width: 1680, height: 1000 },
  deviceScaleFactor: 2,
  serverArgs: ['--mode', 'autonomous', '--width', '64', '--height', '48', '--ticks', '80'],
});

test.beforeEach(async ({page}) => {
  await page.addInitScript(() => {
    // Observe real drawing and its device-to-CSS transform, without changing pixels.
    for (const name of ['clearRect', 'fillRect', 'strokeRect', 'fillText']) {
      const original = CanvasRenderingContext2D.prototype[name];
      CanvasRenderingContext2D.prototype[name] = function(...args) {
        if (this.canvas.id === 'grid') {
          if (name === 'clearRect') window.gridPaint = {cells: [], borders: [], textScales: []};
          const paint = window.gridPaint;
          if (paint && name !== 'clearRect' && paint.cells.length < 16384) {
            const box = this.canvas.getBoundingClientRect(), t = this.getTransform();
            const sx = t.a * box.width / this.canvas.width, sy = t.d * box.height / this.canvas.height;
            if (name === 'fillText') paint.textScales.push([sx, sy]);
            else {
              const [x,y,w,h] = args;
              paint[name === 'fillRect' ? 'cells' : 'borders'].push({x:x*sx,y:y*sy,w:w*sx,h:h*sy});
            }
          }
        }
        return original.apply(this, args);
      };
    }
  });
});

async function checkSquares(page, sample) {
  const paint = await page.evaluate(() => window.gridPaint);
  expect(paint.cells.length).toBe(Math.min(sample.cells.length, 16384));
  const largestCellError = [...paint.cells, ...paint.borders].reduce((error, cell) => Math.max(error, Math.abs(cell.w-cell.h)), 0);
  expect(largestCellError).toBeLessThan(.02);
  const across = paint.cells[1].x - paint.cells[0].x;
  if (sample.width < paint.cells.length) {
    const down = paint.cells[sample.width].y - paint.cells[0].y;
    expect(Math.abs(across-down)).toBeLessThan(.02);
  }
  const largestTextError = paint.textScales.reduce((error, [sx,sy]) => Math.max(error, Math.abs(sx/sy-1)), 0);
  expect(largestTextError).toBeLessThan(.001);
}

async function checkLayout(page, width, portrait = false) {
  // Container resizing schedules one animation-frame repaint, including offline.
  await waitForGridFit(page);
  const layout = await page.evaluate(() => {
    const rect = selector => document.querySelector(selector).getBoundingClientRect();
    const grid = rect('#grid'), world = rect('.world'), toolbar = rect('.toolbar'), frame = rect('#world-frame');
    return {
      viewport: innerWidth, overflow: document.documentElement.scrollWidth > innerWidth,
      grid: { width: grid.width, height: grid.height, left: grid.left, right: grid.right, top:grid.top, bottom:grid.bottom },
      frame: {width:frame.width,height:frame.height,left:frame.left,right:frame.right,top:frame.top,bottom:frame.bottom}, height:innerHeight,
      controlsAbove: toolbar.bottom <= world.top,
      plotWidth: rect('#plot').width,
    };
  });
  expect(layout.viewport).toBe(width);
  expect(layout.overflow).toBe(false);
  expect(layout.grid.width).toBeGreaterThan(0);
  expect(layout.grid.left).toBeGreaterThanOrEqual(12);
  expect(layout.grid.left-layout.frame.left).toBeCloseTo(layout.frame.right-layout.grid.right, 0);
  expect(layout.grid.top).toBeGreaterThanOrEqual(layout.frame.top-.1);
  expect(layout.grid.bottom).toBeLessThanOrEqual(layout.frame.bottom+.1);
  expect(layout.grid.bottom).toBeLessThanOrEqual(layout.height);
  expect(layout.grid.width>layout.frame.width-1 || layout.grid.height>layout.frame.height-1).toBe(true);
  expect(layout.grid.height > layout.grid.width).toBe(portrait);
  expect(layout.controlsAbove).toBe(true);
  expect(layout.plotWidth).toBeLessThanOrEqual(760);
  for (const id of ['grid', 'plot']) {
    if (!await page.locator(`#${id}`).isVisible()) continue; // Hidden plots repaint when their panel opens.
    await expect.poll(() => page.locator(`#${id}`).evaluate(canvas => {
      const box = canvas.getBoundingClientRect();
      const ratio = canvas.id === 'grid' ? Math.min(devicePixelRatio, 4096/box.width, 4096/box.height) : devicePixelRatio;
      return [canvas.width, canvas.height, Math.round(box.width * ratio), Math.round(box.height * ratio)];
    }).then(([w, h, expectedW, expectedH]) => w === expectedW && h === expectedH)).toBe(true);
  }
}

async function clickCell(page, sample, index) {
  expect(index).toBeLessThan(sample.cells.length);
  await clearNarrowInspector(page);
  const cell = await page.evaluate(index => window.gridPaint.cells[index], index);
  await page.locator('#grid').click({position: {x:cell.x+cell.w/2,y:cell.y+cell.h/2}});
}

async function clearNarrowInspector(page) {
  if (await page.evaluate(()=>innerWidth<=720) && await page.locator('#inspection-panel').isVisible()) await page.getByRole('button',{name:'Close inspection',exact:true}).click();
}

async function checkPixels(page, sample) {
  // Sample interior corners in backing pixels, away from letters and selection borders.
  const pixels = await page.locator('#grid').evaluate((canvas, sample) => {
    const ctx = canvas.getContext('2d'), box = canvas.getBoundingClientRect();
    return sample.cells.map((_, i) => {
      const cell = window.gridPaint.cells[i];
      const x = (cell.x + cell.w * .2) * canvas.width / box.width;
      const y = (cell.y + cell.h * .2) * canvas.height / box.height;
      return '#' + [...ctx.getImageData(x, y, 1, 1).data].slice(0, 3).map(n => n.toString(16).padStart(2, '0')).join('');
    });
  }, sample);
  const colors = ['#0072b2', '#d55e00', '#009e73', '#cc79a7'];
  expect(pixels).toEqual(sample.cells.map(agent => agent ? colors[agent.group] : '#f5f7f2'));
}

async function checkLetters(page, sample, visible) {
  // A-D use white ink. Look inside occupied cells, excluding gaps and borders.
  const inkByGroup = await page.locator('#grid').evaluate((canvas, sample) => {
    const ctx = canvas.getContext('2d'), found = [false, false, false, false], box = canvas.getBoundingClientRect();
    sample.cells.forEach((agent, i) => {
      if (!agent) return;
      const cell = window.gridPaint.cells[i];
      const w = cell.w * canvas.width / box.width, h = cell.h * canvas.height / box.height;
      const x = Math.ceil((cell.x + cell.w * .25) * canvas.width / box.width);
      const y = Math.ceil((cell.y + cell.h * .25) * canvas.height / box.height);
      const pixels = ctx.getImageData(x, y, Math.floor(w / 2), Math.floor(h / 2)).data;
      for (let p = 0; p < pixels.length; p += 4) {
        if (pixels[p] > 230 && pixels[p + 1] > 230 && pixels[p + 2] > 230 && pixels[p + 3] === 255) found[agent.group] = true;
      }
    });
    return found;
  }, sample);
  expect(inkByGroup).toEqual(Array(4).fill(visible));
}

async function checkMargins(page, sample, occupied) {
  const canvas = page.locator('#grid'), box = await canvas.boundingBox();
  const cells = await page.evaluate(() => window.gridPaint.cells);
  const first = cells[0], last = cells.at(-1);
  for (const position of [
    {x:first.x/2,y:first.y+first.h/2},
    {x:first.x+first.w/2,y:first.y/2},
    {x:(last.x+last.w+box.width)/2,y:last.y+last.h/2},
    {x:last.x+last.w/2,y:(last.y+last.h+box.height)/2},
  ]) {
    await clearNarrowInspector(page);
    await canvas.click({position});
    await expect(page.locator('#agent')).toHaveValue('');
    await clickCell(page, sample, occupied);
    await expect(page.locator('#agent')).toHaveValue(sample.cells[occupied].id);
  }
}

async function save(page, info, name) {
  const path = info.outputPath(`${name}.png`);
  await page.screenshot({ path, fullPage: true });
  await info.attach(name, { path, contentType: 'image/png' });
}

test('viewport-fit world stays crisp and selection survives paused and offline resizing', async ({page, context, server}, info) => {
  await page.goto(server.url);
  await expect(page.locator('#tick')).toHaveText('0');
  await page.getByRole('button', {name: 'Single step'}).click();
  await expect(page.locator('#tick')).toHaveText('1');
  await expect(page.getByRole('button', {name: 'Single step'})).toBeEnabled();
  const before = await (await page.request.get(`${server.url}/api/snapshot`)).json();
  expect([before.width, before.height]).toEqual([64, 48]);
  await checkSquares(page, before);
  const occupied = before.cells.findIndex(Boolean), empty = before.cells.findIndex(agent => !agent);
  await clickCell(page, before, occupied);
  const selected = before.cells[occupied].id;
  await expect(page.locator('#agent')).toHaveValue(selected);
  const inspection = await page.locator('#inspection').textContent();
  const samples = await page.locator('#samples').textContent();
  const plot = await page.locator('#plot').getAttribute('aria-label');
  await expect(page.locator('#samples')).toContainText('2 samples');
  let writes = 0;
  page.on('request', request => { if (request.method() === 'POST') writes++; });

  await checkLayout(page, 1680);
  await checkSquares(page, before);
  await checkPixels(page, before);
  await checkLetters(page, before, true);
  const pictures = await page.evaluate(() => ['grid', 'plot'].map(id => document.getElementById(id).toDataURL()));
  await save(page, info, 'wide-world-dpr2');
  await page.setViewportSize({width: 1360, height: 900});
  await checkLayout(page, 1360);
  await checkSquares(page, before);
  await checkPixels(page, before);
  await expect(page.locator('#agent')).toHaveValue(selected);
  await expect(page.locator('#inspection')).toHaveText(inspection);
  await expect(page.locator('#samples')).toHaveText(samples);
  await expect(page.locator('#plot')).toHaveAttribute('aria-label', plot);
  await expect(page.locator('#tick')).toHaveText('1');
  expect(await (await page.request.get(`${server.url}/api/snapshot`)).json()).toEqual(before);

  // No successful polling can repaint here: resize must use the retained sample.
  await context.setOffline(true);
  await expect(page.getByRole('alert')).toContainText('Connection lost');
  await page.locator('#agent').evaluate(selector => {
    window.selectorChanges = 0;
    new MutationObserver(() => window.selectorChanges++).observe(selector, {childList: true});
  });
  // At DPR 2 these cells exceed 14 device pixels, but remain too small in CSS pixels.
  await page.setViewportSize({width: 900, height: 900});
  await checkLayout(page, 900);
  await checkLetters(page, before, false);
  await page.setViewportSize({width: 390, height: 844});
  await checkLayout(page, 390);
  await checkSquares(page, before);
  await checkPixels(page, before);
  await checkLetters(page, before, false);
  await expect(page.locator('#agent')).toHaveValue(selected);
  await expect(page.locator('#inspection')).toHaveText(inspection);
  await expect(page.locator('#samples')).toHaveText(samples);
  await expect(page.locator('#plot')).toHaveAttribute('aria-label', plot);
  await expect(page.locator('#tick')).toHaveText('1');
  await expect(page.getByRole('status')).toHaveText('Disconnected');
  await expect(page.getByRole('button', {name: 'Single step'})).toBeDisabled();
  expect(await page.evaluate(() => window.selectorChanges)).toBe(0);
  await save(page, info, 'narrow-world-dpr2-offline');

  // Both empty and occupied hit tests use the resized geometry and real snapshot.
  await clickCell(page, before, empty);
  await expect(page.locator('#agent')).toHaveValue('');
  await expect(page.locator('#inspection')).toHaveText('Select an occupied square or choose an agent above.');
  await clickCell(page, before, occupied);
  await expect(page.locator('#agent')).toHaveValue(selected);
  await expect(page.locator('#inspection')).toHaveText(inspection);
  await checkMargins(page, before, occupied);
  await page.setViewportSize({width: 1680, height: 1000});
  await checkLayout(page, 1680);
  await checkPixels(page, before);
  await checkLetters(page, before, true);
  await context.setOffline(false);
  await expect(page.getByRole('alert')).toBeHidden();
  // Returning to the original size restores the same plot and selection outline.
  await expect.poll(()=>page.evaluate(pictures => ['grid', 'plot'].map((id, i) => document.getElementById(id).toDataURL() === pictures[i]), pictures)).toEqual([true, true]);
  await expect(page.locator('#agent')).toHaveValue(selected);
  await expect(page.locator('#samples')).toHaveText(samples);
  await expect(page.locator('#plot')).toHaveAttribute('aria-label', plot);
  expect(writes).toBe(0);
  await context.setOffline(false);
  await expect(page.getByRole('alert')).toBeHidden();
  expect(await (await page.request.get(`${server.url}/api/snapshot`)).json()).toEqual(before);
});

test.describe('large-world coordinates', () => {
  test.use({
    viewport: {width: 1920, height: 1080}, deviceScaleFactor: 1,
    serverArgs: ['--mode', 'autonomous', '--width', '160', '--height', '100', '--occupancy', '0.05'],
  });
  test('compact world omits numbered axes while retaining exact inspection and history', async ({page, context, server}, info) => {
    await page.addInitScript(() => {
      // Observe real canvas text operations; leave rendering untouched.
      const clear = CanvasRenderingContext2D.prototype.clearRect;
      const fill = CanvasRenderingContext2D.prototype.fillText;
      CanvasRenderingContext2D.prototype.clearRect = function(...args) {
        if (this.canvas.id === 'grid') window.coordinateLabels = [];
        return clear.apply(this, args);
      };
      CanvasRenderingContext2D.prototype.fillText = function(text, x, y, ...args) {
        if (this.canvas.id === 'grid' && /^\d+$/.test(text)) {
          window.coordinateLabels.push(text);
        }
        return fill.call(this, text, x, y, ...args);
      };
    });
    let writes = 0;
    page.on('request', request => { if (request.method() === 'POST') writes++; });
    await page.goto(server.url);
    await expect(page.locator('#tick')).toHaveText('0');
    const before = await (await page.request.get(`${server.url}/api/snapshot`)).json();
    const samples = await page.locator('#samples').textContent();
    const plot = await page.locator('#plot').getAttribute('aria-label');
    const occupied = before.cells.findIndex(Boolean);
    await clickCell(page, before, occupied);
    const selected = before.cells[occupied].id;
    await expect(page.locator('#agent')).toHaveValue(selected);
    const checkCompactFrame = async () => {
      expect(await page.evaluate(() => window.coordinateLabels)).toEqual([]);
      const box = await page.locator('#grid').boundingBox();
      const cells = await page.evaluate(() => window.gridPaint.cells);
      const first = cells[0], last = cells.at(-1);
      // Visible edge space is small and balanced, rather than a reserved axis gutter.
      expect(first.x).toBeLessThan(box.width * .025);
      expect(first.y).toBeCloseTo(first.x, 1);
      expect(box.width - last.x - last.w).toBeCloseTo(first.x, 1);
      expect(box.height - last.y - last.h).toBeCloseTo(first.y, 1);
      await expect(page.locator('#inspection')).toContainText(`Position (${occupied % before.width}, ${Math.floor(occupied / before.width)})`);
    };
    await checkCompactFrame();
    await save(page, info, 'compact-160x100-wide');
    await context.setOffline(true);
    await expect(page.getByRole('alert')).toContainText('Connection lost');
    await page.setViewportSize({width: 390, height: 844});
    await checkLayout(page, 390);
    await checkCompactFrame();
    await expect(page.locator('#tick')).toHaveText('0');
    await expect(page.locator('#agent')).toHaveValue(selected);
    await expect(page.locator('#samples')).toHaveText(samples);
    await expect(page.locator('#plot')).toHaveAttribute('aria-label', plot);
    await save(page, info, 'compact-160x100-narrow');
    expect(writes).toBe(0);
    await context.setOffline(false);
    await expect(page.getByRole('alert')).toBeHidden();
    expect(await (await page.request.get(`${server.url}/api/snapshot`)).json()).toEqual(before);
  });
});

test.describe('portrait world with fractional DPR', () => {
  test.use({viewport:{width:1280,height:900},deviceScaleFactor:1.25,
    serverArgs:['--mode','autonomous','--width','6','--height','11','--ticks','5']});
  test('square cells and undistorted labels retain exact portrait hit targets', async ({page,server}, info) => {
    await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
    const before = await (await page.request.get(`${server.url}/api/snapshot`)).json();
    const occupied = before.cells.findLastIndex(Boolean);
    let writes = 0;
    page.on('request', request => { if (request.method() === 'POST') writes++; });
    for (const width of [1280,1680,700,390]) {
      await page.setViewportSize({width,height:900});
      await checkLayout(page, width, true);
      await checkSquares(page, before);
      await checkPixels(page, before);
      await clickCell(page, before, occupied);
      await expect(page.locator('#agent')).toHaveValue(before.cells[occupied].id);
      await expect(page.locator('#inspection')).toContainText(`Position (${occupied%6}, ${Math.floor(occupied/6)})`);
      await checkMargins(page, before, occupied);
      await checkSquares(page, before);
      await save(page, info, `portrait-${width}-dpr1.25`);
    }
    expect(writes).toBe(0);
    expect(await (await page.request.get(`${server.url}/api/snapshot`)).json()).toEqual(before);
  });
});

test.describe('viewport-fit raster cap',()=>{
  test.use({viewport:{width:1680,height:1000},deviceScaleFactor:8});
  test('high pixel ratios preserve square geometry while capping the backing store',async({page,server})=>{
    await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
    const sample=await (await page.request.get(`${server.url}/api/snapshot`)).json();
    await checkLayout(page,1680); await checkSquares(page,sample); await checkPixels(page,sample);
    const size=await page.locator('#grid').evaluate(canvas=>[canvas.width,canvas.height]);
    expect(Math.max(...size)).toBe(4096);
    expect(size[0]*size[1]).toBeLessThanOrEqual(4096*4096);
  });
});

for (const [width,height] of [[3,87381],[87381,3]]) {
  test.describe(`extreme ${width}x${height} world`, () => {
    test.use({viewport:{width:1280,height:900},deviceScaleFactor:2,
      serverArgs:['--mode','autonomous','--width',String(width),'--height',String(height),'--occupancy','0','--ticks','0']});
    test('supported extreme aspect ratios keep the raster allocation bounded', async ({page,server}) => {
      await page.goto(server.url); await expect(page.locator('#tick')).toHaveText('0');
      await checkLayout(page, 1280, height>width);
      const sample = await (await page.request.get(`${server.url}/api/snapshot`)).json();
      await checkSquares(page, sample);
      const size = await page.locator('#grid').evaluate(canvas => [canvas.width,canvas.height]);
      expect(Math.max(...size)).toBeLessThanOrEqual(4096);
      expect(size[0]*size[1]).toBeLessThanOrEqual(4096*4096);
      // Viewport fitting can keep both extremes below the cap.
    });
  });
}
