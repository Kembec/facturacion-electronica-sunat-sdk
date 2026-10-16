// Page side of browser.mjs: imports the package by name (the import map points it at the file of
// its `default` export condition) and posts the outcome back to the runner.
import { runVectors, selectCases } from './vectors.mjs';

const lines = [];
const report = (body) =>
  fetch('/result', { method: 'POST', body: JSON.stringify({ ...body, lines, userAgent: navigator.userAgent }) });

const read = async (path) => {
  const response = await fetch(`/repo/${path}`);
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
  return response.text();
};

try {
  const { load } = await import('@kembec/facturacion-electronica-sunat-sdk');
  const sdk = await load();
  if ((await load()) !== sdk) throw new Error('load() returns the same SDK on every call');
  const cases = selectCases(await (await fetch('/cases')).json());
  await runVectors({ sdk, cases, read, log: (line) => lines.push(line) });
  await report({ ok: true });
} catch (error) {
  await report({ ok: false, error: String(error?.stack ?? error) });
}
