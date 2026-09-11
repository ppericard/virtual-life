import { test as base, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import path from 'node:path';
import { stripVTControlCharacters } from 'node:util';

async function spawnServer(file, args, onOutput) {
  if (process.platform === 'win32') {
    // Node's child.kill('SIGINT') force-terminates Windows processes. A private
    // terminal lets Ctrl+C reach the real server's console handler instead.
    const pty = await import('node-pty');
    const child = pty.spawn(file, args, { cols: 160, rows: 24, cwd: process.cwd(), env: process.env });
    child.onData(onOutput);
    return {
      exited: new Promise(resolve => child.onExit(({ exitCode, signal }) => resolve({ code: exitCode, signal: signal ?? null }))),
      interrupt: () => child.write('\x03'),
      kill: () => child.kill(),
    };
  }
  const child = spawn(file, args, { stdio: ['ignore', 'pipe', 'pipe'] });
  child.stdout.on('data', onOutput);
  child.stderr.on('data', onOutput);
  return {
    exited: new Promise((resolve, reject) => {
      child.once('exit', (code, signal) => resolve({ code, signal }));
      child.once('error', reject);
    }),
    interrupt: () => child.kill('SIGINT'),
    kill: () => child.kill('SIGKILL'),
  };
}

export const test = base.extend({
  serverArgs: [['--mode', 'demo'], { option: true }],
  server: async ({ serverArgs }, use, testInfo) => {
    let logs = '';
    async function start(port, args = serverArgs) {
      let runLogs = '';
      let reportOutput = () => {};
      const child = await spawnServer(path.resolve('target/debug', process.platform === 'win32' ? 'web.exe' : 'web'), ['--port', String(port), ...args], data => {
        logs += data;
        runLogs += data;
        reportOutput();
      });
      const { exited } = child;
      let hasExited = false;
      exited.then(() => { hasExited = true; }, () => { hasExited = true; });
      const ready = new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error(`Server startup timeout\n${logs}`)), 10000);
        exited.then(() => { clearTimeout(timer); reject(new Error(`Server exited during startup\n${logs}`)); }, error => { clearTimeout(timer); reject(error); });
        reportOutput = () => {
          // Output can arrive in chunks; wait for the whole line, including the port.
          const match = stripVTControlCharacters(runLogs).match(/VirtualLife (http:\/\/127\.0\.0\.1:\d+)\r?\n/);
          if (match) { clearTimeout(timer); resolve(match[1]); }
        };
        reportOutput();
      });
      let stopped = false;
      async function stop() {
        if (stopped) return;
        stopped = true;
        if (!hasExited) child.interrupt();
        let timer;
        let result;
        try { result = await Promise.race([exited, new Promise(resolve => { timer = setTimeout(() => resolve(null), 8000); })]); }
        finally { clearTimeout(timer); }
        if (!result) { child.kill(); throw new Error('Server did not stop cleanly within the shutdown guard'); }
        logs += `Server exit: ${JSON.stringify(result)}\n`;
        expect(result).toEqual({ code: 0, signal: null });
      }
      return { ready, stop };
    }
    let run;
    try {
      run = await start(0);
      const url = await run.ready;
      await use({
        url,
        stop: () => run.stop(),
        restart: async (args = serverArgs) => {
          // Real process replacement at the same origin, never an API reset.
          await run.stop();
          run = await start(new URL(url).port, args);
          expect(await run.ready).toBe(url);
        },
      });
    }
    finally {
      let shutdownFailed = false;
      try { await run?.stop(); }
      catch (error) { shutdownFailed = true; throw error; }
      finally {
        if (shutdownFailed || testInfo.status !== testInfo.expectedStatus) await testInfo.attach('server.log', { body: logs, contentType: 'text/plain' });
      }
    }
  },
});
export { expect };

// Resizing a container schedules a canvas repaint. Wait for that layout before
// measuring click coordinates; Playwright can otherwise click stale offsets.
export async function waitForGridFit(page) {
  await expect.poll(() => page.locator('#grid').evaluate(canvas => {
    const grid = canvas.getBoundingClientRect();
    const frame = canvas.parentElement.getBoundingClientRect();
    return grid.width <= frame.width + .1 && grid.height <= frame.height + .1
      && (grid.width > frame.width - 1 || grid.height > frame.height - 1);
  })).toBe(true);
}

// Follow the production disclosure controls; never force hidden UI into view.
export async function openPanel(page, name) {
  const button = page.getByRole('button', {name, exact: true});
  if (await button.getAttribute('aria-expanded') !== 'true') await button.click();
  await expect(page.locator(`#${await button.getAttribute('aria-controls')}`)).toBeVisible();
}
export async function inspectAgent(page, id) {
  await openPanel(page, 'Inspect');
  await page.locator('#agent').selectOption(id);
}
