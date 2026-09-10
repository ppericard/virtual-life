import { test, expect } from './fixtures.js';

test.use({
  viewport: { width: 1680, height: 1000 },
  deviceScaleFactor: 2,
  serverArgs: ['--mode', 'autonomous', '--width', '64', '--height', '48', '--ticks', '80'],
});

async function checkLayout(page, width) {
  const layout = await page.evaluate(() => {
    const rect = selector => document.querySelector(selector).getBoundingClientRect();
    const grid = rect('#grid'), world = rect('.world'), toolbar = rect('.toolbar');
    return {
      viewport: innerWidth, overflow: document.documentElement.scrollWidth > innerWidth,
      grid: { width: grid.width, height: grid.height, left: grid.left, right: grid.right },
      controlsAbove: toolbar.bottom <= world.top,
      panelsBelow: [...document.querySelectorAll('aside > section')]
        .filter(panel => !panel.hidden).every(panel => panel.getBoundingClientRect().top >= world.bottom),
      plotWidth: rect('#plot').width,
    };
  });
  expect(layout.viewport).toBe(width);
  expect(layout.overflow).toBe(false);
  expect(layout.grid.width).toBeGreaterThan(width - 80);
  expect(layout.grid.left).toBeGreaterThanOrEqual(16);
  expect(width - layout.grid.right).toBeCloseTo(layout.grid.left, 0);
  expect(layout.grid.height).toBeCloseTo(layout.grid.width, 0);
  expect(layout.controlsAbove).toBe(true);
  expect(layout.panelsBelow).toBe(true);
  expect(layout.plotWidth).toBeLessThanOrEqual(760);
  for (const id of ['grid', 'plot']) {
    await expect.poll(() => page.locator(`#${id}`).evaluate(canvas => {
      const box = canvas.getBoundingClientRect();
      return [canvas.width, canvas.height, Math.round(box.width * devicePixelRatio), Math.round(box.height * devicePixelRatio)];
    }).then(([w, h, expectedW, expectedH]) => w === expectedW && h === expectedH)).toBe(true);
  }
}

async function clickCell(page, sample, index) {
  const canvas = page.locator('#grid'), box = await canvas.boundingBox();
  await canvas.click({position: {
    x: (40 + (index % sample.width + .5) * 540 / sample.width) * box.width / 600,
    y: (40 + (Math.floor(index / sample.width) + .5) * 540 / sample.height) * box.height / 600,
  }});
}

async function checkPixels(page, sample) {
  // Sample interior corners in backing pixels, away from letters and selection borders.
  const pixels = await page.locator('#grid').evaluate((canvas, sample) => {
    const ctx = canvas.getContext('2d');
    return sample.cells.map((_, i) => {
      const x = (40 + (i % sample.width + .25) * 540 / sample.width) * canvas.width / 600;
      const y = (40 + (Math.floor(i / sample.width) + .25) * 540 / sample.height) * canvas.height / 600;
      return '#' + [...ctx.getImageData(x, y, 1, 1).data].slice(0, 3).map(n => n.toString(16).padStart(2, '0')).join('');
    });
  }, sample);
  const colors = ['#0072b2', '#d55e00', '#009e73', '#cc79a7'];
  expect(pixels).toEqual(sample.cells.map(agent => agent ? colors[agent.group] : '#f5f7f2'));
}

async function checkLetters(page, sample, visible) {
  // A-D use white ink. Look inside occupied cells, excluding gaps and borders.
  const inkByGroup = await page.locator('#grid').evaluate((canvas, sample) => {
    const ctx = canvas.getContext('2d'), found = [false, false, false, false];
    sample.cells.forEach((agent, i) => {
      if (!agent) return;
      const w = 540 / sample.width * canvas.width / 600;
      const h = 540 / sample.height * canvas.height / 600;
      const x = Math.ceil(40 * canvas.width / 600 + (i % sample.width + .25) * w);
      const y = Math.ceil(40 * canvas.height / 600 + (Math.floor(i / sample.width) + .25) * h);
      const pixels = ctx.getImageData(x, y, Math.floor(w / 2), Math.floor(h / 2)).data;
      for (let p = 0; p < pixels.length; p += 4) {
        if (pixels[p] > 230 && pixels[p + 1] > 230 && pixels[p + 2] > 230 && pixels[p + 3] === 255) found[agent.group] = true;
      }
    });
    return found;
  }, sample);
  expect(inkByGroup).toEqual(Array(4).fill(visible));
}

async function save(page, info, name) {
  const path = info.outputPath(`${name}.png`);
  await page.screenshot({ path, fullPage: true });
  await info.attach(name, { path, contentType: 'image/png' });
}

test('full-width world stays crisp and selection survives paused and offline resizing', async ({page, context, server}, info) => {
  await page.goto(server.url);
  await expect(page.locator('#tick')).toHaveText('0');
  await page.getByRole('button', {name: 'Single step'}).click();
  await expect(page.locator('#tick')).toHaveText('1');
  await expect(page.getByRole('button', {name: 'Single step'})).toBeEnabled();
  const before = await (await page.request.get(`${server.url}/api/snapshot`)).json();
  expect([before.width, before.height]).toEqual([64, 48]);
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
  await checkPixels(page, before);
  await checkLetters(page, before, true);
  const pictures = await page.evaluate(() => ['grid', 'plot'].map(id => document.getElementById(id).toDataURL()));
  await save(page, info, 'wide-world-dpr2');
  await page.setViewportSize({width: 1360, height: 900});
  await checkLayout(page, 1360);
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
  await page.setViewportSize({width: 1680, height: 1000});
  await checkLayout(page, 1680);
  await checkPixels(page, before);
  await checkLetters(page, before, true);
  // Returning to the original size restores the same plot and selection outline.
  expect(await page.evaluate(pictures => ['grid', 'plot'].map((id, i) => document.getElementById(id).toDataURL() === pictures[i]), pictures)).toEqual([true, true]);
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
  test('coordinate labels leave readable gaps without changing the observed world', async ({page, context, server}, info) => {
    await page.addInitScript(() => {
      // Observe real canvas text operations and font metrics; leave rendering untouched.
      const clear = CanvasRenderingContext2D.prototype.clearRect;
      const fill = CanvasRenderingContext2D.prototype.fillText;
      CanvasRenderingContext2D.prototype.clearRect = function(...args) {
        if (this.canvas.id === 'grid') window.coordinateLabels = [];
        return clear.apply(this, args);
      };
      CanvasRenderingContext2D.prototype.fillText = function(text, x, y, ...args) {
        if (this.canvas.id === 'grid' && /^\d+$/.test(text)) {
          const metrics = this.measureText(text), box = this.canvas.getBoundingClientRect();
          const transform = this.getTransform();
          const sx = transform.a * box.width / this.canvas.width;
          const sy = transform.d * box.height / this.canvas.height;
          window.coordinateLabels.push({text, axis: y < 40 ? 'x' : 'y',
            left: (x - metrics.actualBoundingBoxLeft) * sx,
            right: (x + metrics.actualBoundingBoxRight) * sx,
            top: (y - metrics.actualBoundingBoxAscent) * sy,
            bottom: (y + metrics.actualBoundingBoxDescent) * sy,
          });
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
    const checkLabels = async () => {
      const labels = await page.evaluate(() => window.coordinateLabels);
      const box = await page.locator('#grid').boundingBox();
      const horizontal = labels.filter(label => label.axis === 'x');
      const vertical = labels.filter(label => label.axis === 'y');
      expect(horizontal.some(label => Number(label.text) >= 100)).toBe(true);
      expect(vertical.length).toBeGreaterThan(1);
      for (const label of labels) {
        expect(label.left).toBeGreaterThanOrEqual(0);
        expect(label.right).toBeLessThanOrEqual(box.width);
        expect(label.top).toBeGreaterThanOrEqual(0);
        expect(label.bottom).toBeLessThanOrEqual(box.height);
      }
      for (let i = 1; i < horizontal.length; i++) {
        expect(horizontal[i].left - horizontal[i - 1].right,
          `gap between x labels ${horizontal[i - 1].text} and ${horizontal[i].text}`)
          .toBeGreaterThanOrEqual(4 * box.width / 600);
      }
      for (let i = 1; i < vertical.length; i++) {
        expect(vertical[i].top - vertical[i - 1].bottom).toBeGreaterThanOrEqual(4 * box.height / 600);
      }
    };
    await checkLabels();
    await save(page, info, 'coordinates-160x100-wide');
    await context.setOffline(true);
    await expect(page.getByRole('alert')).toContainText('Connection lost');
    await page.setViewportSize({width: 390, height: 844});
    await checkLayout(page, 390);
    await checkLabels();
    await expect(page.locator('#tick')).toHaveText('0');
    await expect(page.locator('#agent')).toHaveValue(selected);
    await expect(page.locator('#samples')).toHaveText(samples);
    await expect(page.locator('#plot')).toHaveAttribute('aria-label', plot);
    await save(page, info, 'coordinates-160x100-narrow');
    expect(writes).toBe(0);
    await context.setOffline(false);
    await expect(page.getByRole('alert')).toBeHidden();
    expect(await (await page.request.get(`${server.url}/api/snapshot`)).json()).toEqual(before);
  });
});
