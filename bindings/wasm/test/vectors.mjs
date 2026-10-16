// Every vector through the package API, shared by the Node and the browser runs: vouchers (01, 03,
// 07, 08), despatch advices (09), daily summaries (rc-) and void communications (ra-). Each fixture
// is built and signed and must carry the DigestValue and SignatureValue of its vector. No Node
// imports: it runs in a page.

export const EXPECTED_CASES = 34;

/** Fixture names (without `.json`), sorted. */
export function selectCases(fileNames) {
  return fileNames
    .filter((name) => /^(0[13789]|ra|rc)-.*\.json$/.test(name))
    .map((name) => name.slice(0, -'.json'.length))
    .sort();
}

function check(condition, message) {
  if (!condition) throw new Error(`check failed: ${message}`);
}

function elementText(xml, element) {
  const match = xml.match(new RegExp(`<${element}>([^<]*)<`));
  check(match, `no ${element}`);
  return match[1].replace(/\s+/g, '');
}

function throwsError(run, message) {
  try {
    run();
  } catch (error) {
    check(error instanceof Error && error.message.length > 0, `${message}: throws an Error with a message`);
    return;
  }
  check(false, `${message}: did not throw`);
}

/**
 * @param {{ sdk: import('../npm/index.d.ts').Sdk, cases: string[], read: (path: string) => Promise<string>, log: (line: string) => void }} run
 */
export async function runVectors({ sdk, cases, read, log }) {
  check(cases.length === EXPECTED_CASES, `${EXPECTED_CASES} fixtures, found ${cases.length}`);
  const cert = await read('tests/vectors/test-cert/test-cert.pem');
  const key = await read('tests/vectors/test-cert/test-key.pem');

  for (const name of cases) {
    const built = JSON.parse(sdk.build(await read(`fixtures/${name}.json`)));
    check(typeof built.file_name === 'string' && built.file_name.length > 0, `${name}: file_name`);
    const signed = JSON.parse(sdk.sign(built.xml, cert, key));
    const expected = await read(`tests/vectors/${name}.signed.xml`);
    check(signed.digest_value === elementText(expected, 'ds:DigestValue'), `${name}: digest_value`);
    check(signed.signature_value === elementText(expected, 'ds:SignatureValue'), `${name}: signature_value`);
    check(signed.xml.includes('Id="signatureKG"'), `${name}: default signature_id`);
    log(`ok ${name}`);
  }

  const unsigned = await read(`tests/vectors/${cases[0]}.unsigned.xml`);
  const custom = JSON.parse(sdk.sign(unsigned, cert, key, 'customSig'));
  check(custom.xml.includes('Id="customSig"'), 'custom signature_id');
  throwsError(() => sdk.build('{not json'), 'invalid JSON');
  throwsError(() => sdk.sign('<a/>', cert, 'not a key'), 'invalid key');
  log(`ok ${cases.length} fixtures built and signed like their vectors`);
}
