// Runs the voucher vectors in headless Chrome through the installed package: serves a page whose
// import map points the package name at the file of its `default` export condition (web.js), so the
// .wasm is fetched by URL as in a real page, and waits for the page to post its outcome.
// Usage: node <consumer>/browser.mjs <repository root>   (CHROME_BIN overrides the browser lookup)
import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { createServer } from 'node:http';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { dirname, extname, join, normalize, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const PACKAGE = '@kembec/facturacion-electronica-sunat-sdk';
const TIMEOUT_MS = 120_000;
const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.wasm': 'application/wasm',
};

const root = process.argv[2];
if (!root) throw new Error('usage: node browser.mjs <repository root>');
const here = dirname(fileURLToPath(import.meta.url));
const packageDir = dirname(createRequire(import.meta.url).resolve(`${PACKAGE}/package.json`));
const entry = JSON.parse(readFileSync(join(packageDir, 'package.json'), 'utf8')).exports['.'].default;
const WASM_PATH = '/pkg/wasm/sunat_sdk_wasm_bg.wasm';

const page = `<!doctype html><meta charset="utf-8"><title>sdk</title>
<script type="importmap">${JSON.stringify({ imports: { [PACKAGE]: `/pkg/${entry.replace(/^\.\//, '')}` } })}</script>
<script type="module" src="/browser-page.mjs"></script>`;

function chromeBinary() {
  if (process.env.CHROME_BIN) return process.env.CHROME_BIN;
  const mac = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
  if (existsSync(mac)) return mac;
  for (const name of ['google-chrome', 'google-chrome-stable', 'chromium', 'chromium-browser']) {
    if (spawnSync('which', [name]).status === 0) return name;
  }
  throw new Error('no Chrome found: set CHROME_BIN');
}

/** Absolute path of `relative` inside `base`, or undefined if it escapes it. */
function inside(base, relative) {
  const path = normalize(join(base, decodeURIComponent(relative)));
  return path.startsWith(base + sep) ? path : undefined;
}

const served = new Set();
let settle;
const outcome = new Promise((resolve) => (settle = resolve));

const server = createServer((request, response) => {
  const url = new URL(request.url, 'http://localhost');
  served.add(url.pathname);
  const send = (status, body, type = 'text/plain; charset=utf-8') => {
    response.writeHead(status, { 'content-type': type, 'cache-control': 'no-store' });
    response.end(body);
  };
  if (request.method === 'POST' && url.pathname === '/result') {
    let body = '';
    request.on('data', (chunk) => (body += chunk));
    request.on('end', () => {
      send(204, '');
      settle(JSON.parse(body));
    });
    return;
  }
  if (url.pathname === '/') return send(200, page, TYPES['.html']);
  if (url.pathname === '/cases') return send(200, JSON.stringify(readdirSync(join(root, 'fixtures'))), TYPES['.json']);
  const [, scope, ...rest] = url.pathname.split('/');
  const base = { pkg: packageDir, repo: root }[scope];
  const file = base ? inside(base, rest.join('/')) : inside(here, [scope, ...rest].join('/'));
  if (!file || !existsSync(file)) return send(404, `not found: ${url.pathname}`);
  send(200, readFileSync(file), TYPES[extname(file)] ?? 'text/plain; charset=utf-8');
});

await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
const address = `http://127.0.0.1:${server.address().port}/`;
const profile = mkdtempSync(join(tmpdir(), 'sdk-chrome-'));
// --no-sandbox: CI runners lack the user namespaces Chrome's sandbox needs; the page is our own.
const chrome = spawn(
  chromeBinary(),
  ['--headless=new', '--no-sandbox', '--disable-gpu', '--no-first-run', '--no-default-browser-check',
    `--user-data-dir=${profile}`, address],
  { stdio: ['ignore', 'ignore', 'pipe'] },
);
let chromeLog = '';
chrome.stderr.on('data', (chunk) => (chromeLog += chunk));
chrome.on('exit', (code) => settle({ ok: false, error: `Chrome exited (${code}) before the page reported\n${chromeLog}` }));
const timer = setTimeout(() => settle({ ok: false, error: `no result within ${TIMEOUT_MS} ms` }), TIMEOUT_MS);

const result = await outcome;
clearTimeout(timer);
chrome.removeAllListeners('exit');
chrome.kill();
server.close();
rmSync(profile, { recursive: true, force: true });

for (const line of result.lines ?? []) console.log(line);
if (!result.ok) {
  console.error(result.error);
  process.exit(1);
}
if (!served.has(WASM_PATH)) {
  console.error(`the page never fetched ${WASM_PATH}: load() did not use its default URL`);
  process.exit(1);
}
console.log(`ok browser ${result.userAgent} via /pkg/${entry.replace(/^\.\//, '')}`);
