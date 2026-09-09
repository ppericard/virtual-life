import { test, expect } from './fixtures.js';

for (const status of ['paused', 'running']) {
  test.describe(`Ctrl+C while ${status}`, () => {
    test.use({ serverArgs: status === 'running' ? ['--running', '--tick-ms', '60000'] : [] });

    test('the real server exits cleanly and releases its listener', async ({ server, request }) => {
      // An actual reply establishes that the server is ready, without a startup sleep.
      const response = await request.get(`${server.url}/api/snapshot`);
      expect(response.ok()).toBe(true);
      expect(await response.json()).toMatchObject({ status, tick: '0' });
      // stop() requires exit code 0 and fails on forced termination or a timeout.
      await server.stop();
      await expect(request.get(`${server.url}/api/snapshot`, { timeout: 1000 })).rejects.toThrow();
    });
  });
}
