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
  serverArgs: [[], { option: true }],
  server: async ({ serverArgs }, use, testInfo) => {
    let logs = '';
    let reportOutput = () => {};
    const child = await spawnServer(path.resolve('target/debug', process.platform === 'win32' ? 'web.exe' : 'web'), ['--port', '0', ...serverArgs], data => {
      logs += data;
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
        const match = stripVTControlCharacters(logs).match(/VirtualLife (http:\/\/127\.0\.0\.1:\d+)\r?\n/);
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
      expect(result).toEqual({ code: 0, signal: null });
    }
    try { await use({ url: await ready, stop }); }
    finally {
      let shutdownFailed = false;
      try { await stop(); }
      catch (error) { shutdownFailed = true; throw error; }
      finally {
        if (shutdownFailed || testInfo.status !== testInfo.expectedStatus) await testInfo.attach('server.log', { body: logs, contentType: 'text/plain' });
      }
    }
  },
});
export { expect };
