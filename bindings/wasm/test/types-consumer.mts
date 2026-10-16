// Type-checked (never run) from the clean consumer: the declarations resolve through `exports`
// and describe the API that node-consumer.mjs and browser-page.mjs exercise.
import { load, type BuildOutput, type Sdk, type SignOutput } from '@kembec/facturacion-electronica-sunat-sdk';
import wasm from '@kembec/facturacion-electronica-sunat-sdk/wasm';

const sdk: Sdk = await load();
const same: Sdk = await load(wasm);
const fromUrl: Promise<Sdk> = load(new URL('https://example.com/sunat_sdk_wasm_bg.wasm'));
const built: BuildOutput = JSON.parse(sdk.build('{}'));
const signed: SignOutput = JSON.parse(same.sign(built.xml, 'cert', 'key'));
const named: string = sdk.sign(built.xml, 'cert', 'key', 'customSig');
export const used = [fromUrl, signed.digest_value, signed.signature_value, built.file_name, named];

// @ts-expect-error signatureId is a string
sdk.sign(built.xml, 'cert', 'key', 1);
// @ts-expect-error the SDK has only build and sign
sdk.send();
