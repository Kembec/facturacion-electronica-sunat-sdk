* Builds the test invoice of the zip, signs it with the self-generated test certificate (valid for
* no taxpayer) and shows the file name and the DigestValue, which must equal the vector's.
*   DO sample.prg
LOCAL lcRoot, lcBuilt, lcSigned, lcRequest
lcRoot = ADDBS(JUSTPATH(SYS(16))) + "..\"
SET PROCEDURE TO (ADDBS(JUSTPATH(SYS(16))) + "sunat_sdk.prg") ADDITIVE
sunat_load(m.lcRoot + "dll\x86\sunat_sdk_ffi.dll")

* FILETOSTR keeps the file's UTF-8 bytes, which is what the DLL reads.
lcBuilt = sunat_call('{"command":"build","input":' + FILETOSTR(m.lcRoot + "sample\01-invoice-taxed-cash.json") + '}')
lcRequest = '{"command":"sign","flags":{"cert":' + sunat_json_quote(sunat_to_utf8(m.lcRoot + "sample\test-cert.pem")) + ;
	',"key":' + sunat_json_quote(sunat_to_utf8(m.lcRoot + "sample\test-key.pem")) + '},' + ;
	'"input":{"xml":' + sunat_json_get(m.lcBuilt, "xml") + '}}'
* The XML goes to "sign" as the JSON string "build" gave, without decoding it: nothing to lose and
* no character-by-character loop. sunat_json_text(m.lcBuilt, "xml") is the XML itself, to store it.
lcSigned = sunat_call(m.lcRequest)

* Expected: 20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=
MESSAGEBOX(sunat_json_text(m.lcBuilt, "file_name") + " " + sunat_json_text(m.lcSigned, "digest_value"), 64, ;
	"SUNAT SDK " + sunat_version())
