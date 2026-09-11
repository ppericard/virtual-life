// Throwaway, loopback-only static preview. No production routes or persistence.
import {createServer} from 'node:http';
import {readFileSync} from 'node:fs';
const files = new Map([
  ['/', ['index.html','text/html; charset=utf-8']],
  ['/app.js', ['app.js','text/javascript; charset=utf-8']],
  ['/model.js', ['model.js','text/javascript; charset=utf-8']],
  ['/style.css', ['style.css','text/css; charset=utf-8']],
]);
const port = Number(process.env.MATERIAL_PROTOTYPE_PORT || 7881);
const server = createServer((req, res) => {
  const file = files.get(new URL(req.url, 'http://localhost').pathname);
  if (req.method !== 'GET' || !file) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, {'Content-Type':file[1], 'Cache-Control':'no-store',
    'Content-Security-Policy':"default-src 'self'; style-src 'self'; script-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"});
  res.end(readFileSync(new URL(file[0], import.meta.url)));
});
server.listen(port, '127.0.0.1', () => process.stdout.write(`Material prototype: http://127.0.0.1:${port}/\nCtrl+C stops this prototype only.\n`));
process.on('SIGINT', () => server.close(() => process.exit(0)));
