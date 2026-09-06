import { test as base, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import path from 'node:path';

export const test = base.extend({
  serverArgs: [[], { option: true }],
  server: async ({ serverArgs }, use, testInfo) => {
    const child = spawn(path.resolve('target/debug', process.platform === 'win32' ? 'web.exe' : 'web'), ['--port', '0', ...serverArgs], { stdio: ['ignore', 'pipe', 'pipe'] });
    let logs = '';
    child.stderr.on('data', data => { logs += data; });
    const exited = new Promise(resolve => child.once('exit', (code, signal) => resolve({ code, signal })));
    const ready = new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error(`Server startup timeout\n${logs}`)), 10000);
      child.once('error', error => { clearTimeout(timer); reject(error); });
      child.once('exit', () => { clearTimeout(timer); reject(new Error(`Server exited during startup\n${logs}`)); });
      child.stdout.on('data', data => {
        logs += data;
        const match = logs.match(/VirtualLife (http:\/\/127\.0\.0\.1:\d+)/);
        if (match) { clearTimeout(timer); resolve(match[1]); }
      });
    });
    let stopped = false;
    async function stop() {
      if (stopped) return;
      stopped = true;
      child.kill('SIGINT');
      let timer;
      const result = await Promise.race([exited, new Promise(resolve => { timer = setTimeout(() => resolve(null), 8000); })]);
      clearTimeout(timer);
      if (!result) { child.kill('SIGKILL'); throw new Error('Server did not stop cleanly within the shutdown guard'); }
      expect(result).toEqual({ code: 0, signal: null });
    }
    try { await use({ url: await ready, stop }); }
    finally {
      await stop();
      if (testInfo.status !== testInfo.expectedStatus) await testInfo.attach('server.log', { body: logs, contentType: 'text/plain' });
    }
  },
});
export { expect };
