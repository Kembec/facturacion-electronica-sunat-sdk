* SUNAT electronic invoicing SDK for Visual FoxPro 9, through the C ABI of sunat_sdk_ffi.dll
* (docs/c-abi.md).
*
*   SET PROCEDURE TO sunat_sdk.prg ADDITIVE
*   sunat_load("C:\miapp\dll\x86\sunat_sdk_ffi.dll")
*   lcBuilt = sunat_call('{"command":"build","input":' + FILETOSTR("factura.json") + '}')
*   lcXml = sunat_json_text(lcBuilt, "xml")
*
* Every string here is UTF-8, as the DLL speaks it. VFP strings are bytes, so UTF-8 passes through
* untouched and the JSON delimiters are ASCII. Convert at the edges only: sunat_to_utf8() for text
* that comes from a table or a form (the ANSI code page), sunat_from_utf8() to show a value. A
* signed XML taken through the ANSI code page and back loses any character outside it.
*
* The response is copied into a VFP string with sunat_sdk_response_len and sunat_sdk_response_copy,
* because VFP cannot read through a pointer, and released with sunat_sdk_free right after.
*
* bindings/legacy/tests/declarations.py reads the six DECLARE lines of sunat_load and the CI calls
* the 32-bit DLL through exactly these types; keep each DECLARE on one line.

#DEFINE SUNAT_SDK_OK 0
#DEFINE SUNAT_SDK_ABI_VERSION 1
#DEFINE HEX_DIGITS "0123456789ABCDEF"

* Declares the DLL's functions from a full path and checks its ABI.
PROCEDURE sunat_load
	LPARAMETERS tcDll
	IF NOT FILE(m.tcDll)
		ERROR "sunat_sdk: " + m.tcDll + " not found; Visual FoxPro needs the 32-bit DLL, dll\x86"
	ENDIF
	* stdcall, the default of DECLARE, with undecorated names. Names are case-sensitive.
	DECLARE INTEGER sunat_sdk_call IN (m.tcDll) STRING request, INTEGER @ response
	DECLARE INTEGER sunat_sdk_response_len IN (m.tcDll) INTEGER response
	DECLARE INTEGER sunat_sdk_response_copy IN (m.tcDll) INTEGER response, STRING @ buffer, INTEGER capacity
	DECLARE sunat_sdk_free IN (m.tcDll) INTEGER response
	DECLARE STRING sunat_sdk_version IN (m.tcDll)
	DECLARE INTEGER sunat_sdk_abi_version IN (m.tcDll)
	IF sunat_sdk_abi_version() <> SUNAT_SDK_ABI_VERSION
		ERROR "sunat_sdk: sunat_sdk_ffi.dll is of another ABI version"
	ENDIF
ENDPROC

* The SDK version of the loaded DLL, for example "1.0.0-rc.9".
FUNCTION sunat_version
	RETURN sunat_sdk_version()
ENDFUNC

* Runs one command (docs/c-abi.md) with a UTF-8 JSON request and returns the UTF-8 JSON response.
* A rejected command raises error 1098 with the SDK's message. A SOAP fault or a query without
* verdict is not an error: it is the "outcome" of the response, as in the CLI.
FUNCTION sunat_call
	LPARAMETERS tcRequest
	LOCAL lnResponse, lnStatus, lnLength, lcBuffer, lnCopied
	lnResponse = 0
	* The NUL ends the request for the DLL; the request is only read during the call.
	lnStatus = sunat_sdk_call(m.tcRequest + CHR(0), @lnResponse)
	IF m.lnResponse = 0
		ERROR "sunat_sdk: sunat_sdk_call gave no response"
	ENDIF
	lnLength = sunat_sdk_response_len(m.lnResponse)
	lcBuffer = REPLICATE(CHR(0), m.lnLength + 1)
	lnCopied = sunat_sdk_response_copy(m.lnResponse, @lcBuffer, m.lnLength + 1)
	= sunat_sdk_free(m.lnResponse)
	IF m.lnCopied <> SUNAT_SDK_OK
		ERROR "sunat_sdk: sunat_sdk_response_copy failed"
	ENDIF
	lcBuffer = LEFT(m.lcBuffer, m.lnLength)
	IF m.lnStatus <> SUNAT_SDK_OK
		ERROR "sunat_sdk: " + sunat_from_utf8(sunat_json_text(m.lcBuffer, "error"))
	ENDIF
	RETURN m.lcBuffer
ENDFUNC

* Text of the current code page (a table field, a form) as UTF-8.
FUNCTION sunat_to_utf8
	LPARAMETERS tcText
	RETURN STRCONV(m.tcText, 9, CPCURRENT(), 1)
ENDFUNC

* UTF-8 as text of the current code page, to show it or store it in a table.
FUNCTION sunat_from_utf8
	LPARAMETERS tcUtf8
	RETURN STRCONV(m.tcUtf8, 11, CPCURRENT(), 1)
ENDFUNC

* ---------------------------------------------------------------------------------------------
* JSON, without COM components: a port of vb6/SunatJson.bas (which the CI runs over the 38
* responses) to bytes. Values are read as raw JSON text and decoded on demand:
*   sunat_json_text(sunat_json_get(lcResponse, "cdr"), "code")

* The raw JSON text of member tcKey of the object tcJson, or "" when it has no such member.
FUNCTION sunat_json_get
	LPARAMETERS tcJson, tcKey
	LOCAL lnPos, lnKeyStart, lnKeyEnd, lnValueStart, lnValueEnd
	lnPos = json_skip_space(m.tcJson, 1)
	IF SUBSTR(m.tcJson, m.lnPos, 1) <> "{"
		ERROR "sunat_sdk: not a JSON object"
	ENDIF
	lnPos = json_skip_space(m.tcJson, m.lnPos + 1)
	DO WHILE SUBSTR(m.tcJson, m.lnPos, 1) == '"'
		lnKeyStart = m.lnPos
		lnKeyEnd = json_skip_string(m.tcJson, m.lnPos)
		lnPos = json_skip_space(m.tcJson, m.lnKeyEnd)
		IF SUBSTR(m.tcJson, m.lnPos, 1) <> ":"
			ERROR "sunat_sdk: expected ':' in a JSON object"
		ENDIF
		lnValueStart = json_skip_space(m.tcJson, m.lnPos + 1)
		lnValueEnd = json_skip_value(m.tcJson, m.lnValueStart)
		IF sunat_json_decode(SUBSTR(m.tcJson, m.lnKeyStart, m.lnKeyEnd - m.lnKeyStart)) == m.tcKey
			RETURN SUBSTR(m.tcJson, m.lnValueStart, m.lnValueEnd - m.lnValueStart)
		ENDIF
		lnPos = json_skip_space(m.tcJson, m.lnValueEnd)
		IF SUBSTR(m.tcJson, m.lnPos, 1) == ","
			lnPos = json_skip_space(m.tcJson, m.lnPos + 1)
		ENDIF
	ENDDO
	RETURN ""
ENDFUNC

* Member tcKey decoded: a string without its quotes and escapes, a number or true/false as
* written, "" for null or a missing member.
FUNCTION sunat_json_text
	LPARAMETERS tcJson, tcKey
	RETURN sunat_json_decode(sunat_json_get(m.tcJson, m.tcKey))
ENDFUNC

* Number of elements of the array tcJson.
FUNCTION sunat_json_count
	LPARAMETERS tcJson
	LOCAL lnPos, lnCount
	lnPos = json_array_start(m.tcJson)
	lnCount = 0
	DO WHILE m.lnPos <= LEN(m.tcJson) AND SUBSTR(m.tcJson, m.lnPos, 1) <> "]"
		lnCount = m.lnCount + 1
		lnPos = json_next_item(m.tcJson, json_skip_value(m.tcJson, m.lnPos))
	ENDDO
	RETURN m.lnCount
ENDFUNC

* The raw JSON text of element tnIndex (from 0) of the array tcJson, or "" past its end.
FUNCTION sunat_json_item
	LPARAMETERS tcJson, tnIndex
	LOCAL lnPos, lnEnd, lnCurrent
	lnPos = json_array_start(m.tcJson)
	lnCurrent = 0
	DO WHILE m.lnPos <= LEN(m.tcJson) AND SUBSTR(m.tcJson, m.lnPos, 1) <> "]"
		lnEnd = json_skip_value(m.tcJson, m.lnPos)
		IF m.lnCurrent = m.tnIndex
			RETURN SUBSTR(m.tcJson, m.lnPos, m.lnEnd - m.lnPos)
		ENDIF
		lnCurrent = m.lnCurrent + 1
		lnPos = json_next_item(m.tcJson, m.lnEnd)
	ENDDO
	RETURN ""
ENDFUNC

* A raw JSON value as UTF-8 text: a string loses its quotes and escapes, null becomes "", anything
* else (number, true, false, object, array) is returned as written.
FUNCTION sunat_json_decode
	LPARAMETERS tcToken
	LOCAL lcBody, lcOut, lnPos, lnLen, lcChar, lnCode, lnLow
	IF m.tcToken == "null"
		RETURN ""
	ENDIF
	IF LEFT(m.tcToken, 1) <> '"'
		RETURN m.tcToken
	ENDIF
	lcBody = SUBSTR(m.tcToken, 2, LEN(m.tcToken) - 2)
	lnLen = LEN(m.lcBody)
	lcOut = ""
	lnPos = 1
	DO WHILE m.lnPos <= m.lnLen
		lcChar = SUBSTR(m.lcBody, m.lnPos, 1)
		IF m.lcChar <> "\"
			lcOut = m.lcOut + m.lcChar
			lnPos = m.lnPos + 1
			LOOP
		ENDIF
		lcChar = SUBSTR(m.lcBody, m.lnPos + 1, 1)
		lnPos = m.lnPos + 2
		DO CASE
		CASE m.lcChar == "n"
			lcOut = m.lcOut + CHR(10)
		CASE m.lcChar == "r"
			lcOut = m.lcOut + CHR(13)
		CASE m.lcChar == "t"
			lcOut = m.lcOut + CHR(9)
		CASE m.lcChar == "b"
			lcOut = m.lcOut + CHR(8)
		CASE m.lcChar == "f"
			lcOut = m.lcOut + CHR(12)
		CASE m.lcChar == "u"
			lnCode = json_hex(SUBSTR(m.lcBody, m.lnPos, 4))
			lnPos = m.lnPos + 4
			* A surrogate pair (\uD83D\uDE00) is one code point.
			IF BETWEEN(m.lnCode, 0xD800, 0xDBFF) AND SUBSTR(m.lcBody, m.lnPos, 2) == "\u"
				lnLow = json_hex(SUBSTR(m.lcBody, m.lnPos + 2, 4))
				lnCode = 0x10000 + (m.lnCode - 0xD800) * 0x400 + (m.lnLow - 0xDC00)
				lnPos = m.lnPos + 6
			ENDIF
			lcOut = m.lcOut + json_utf8(m.lnCode)
		OTHERWISE
			lcOut = m.lcOut + m.lcChar
		ENDCASE
	ENDDO
	RETURN m.lcOut
ENDFUNC

* tcText (UTF-8) as a JSON string literal, quotes included. Bytes above 127 stay as they are.
FUNCTION sunat_json_quote
	LPARAMETERS tcText
	LOCAL lcOut, lnPos, lnCode
	lcOut = '"'
	FOR lnPos = 1 TO LEN(m.tcText)
		lnCode = ASC(SUBSTR(m.tcText, m.lnPos, 1))
		DO CASE
		CASE m.lnCode = 34
			lcOut = m.lcOut + '\"'
		CASE m.lnCode = 92
			lcOut = m.lcOut + "\\"
		CASE m.lnCode = 10
			lcOut = m.lcOut + "\n"
		CASE m.lnCode = 13
			lcOut = m.lcOut + "\r"
		CASE m.lnCode = 9
			lcOut = m.lcOut + "\t"
		CASE m.lnCode < 32
			lcOut = m.lcOut + "\u00" + SUBSTR(HEX_DIGITS, INT(m.lnCode / 16) + 1, 1) + SUBSTR(HEX_DIGITS, MOD(m.lnCode, 16) + 1, 1)
		OTHERWISE
			lcOut = m.lcOut + CHR(m.lnCode)
		ENDCASE
	ENDFOR
	RETURN m.lcOut + '"'
ENDFUNC

* A code point as UTF-8 bytes.
FUNCTION json_utf8
	LPARAMETERS tnCode
	DO CASE
	CASE m.tnCode < 0x80
		RETURN CHR(m.tnCode)
	CASE m.tnCode < 0x800
		RETURN CHR(0xC0 + INT(m.tnCode / 0x40)) + CHR(0x80 + MOD(m.tnCode, 0x40))
	CASE m.tnCode < 0x10000
		RETURN CHR(0xE0 + INT(m.tnCode / 0x1000)) + CHR(0x80 + MOD(INT(m.tnCode / 0x40), 0x40)) + CHR(0x80 + MOD(m.tnCode, 0x40))
	OTHERWISE
		RETURN CHR(0xF0 + INT(m.tnCode / 0x40000)) + CHR(0x80 + MOD(INT(m.tnCode / 0x1000), 0x40)) + ;
			CHR(0x80 + MOD(INT(m.tnCode / 0x40), 0x40)) + CHR(0x80 + MOD(m.tnCode, 0x40))
	ENDCASE
ENDFUNC

FUNCTION json_hex
	LPARAMETERS tcDigits
	LOCAL lnValue, lnPos, lnDigit
	lnValue = 0
	FOR lnPos = 1 TO 4
		lnDigit = AT(UPPER(SUBSTR(m.tcDigits, m.lnPos, 1)), HEX_DIGITS) - 1
		IF m.lnDigit < 0 OR LEN(m.tcDigits) <> 4
			ERROR "sunat_sdk: bad \u escape in JSON"
		ENDIF
		lnValue = m.lnValue * 16 + m.lnDigit
	ENDFOR
	RETURN m.lnValue
ENDFUNC

FUNCTION json_array_start
	LPARAMETERS tcJson
	LOCAL lnPos
	lnPos = json_skip_space(m.tcJson, 1)
	IF SUBSTR(m.tcJson, m.lnPos, 1) <> "["
		ERROR "sunat_sdk: not a JSON array"
	ENDIF
	RETURN json_skip_space(m.tcJson, m.lnPos + 1)
ENDFUNC

* Position of the next element after a value that ends at tnPos.
FUNCTION json_next_item
	LPARAMETERS tcJson, tnPos
	LOCAL lnPos
	lnPos = json_skip_space(m.tcJson, m.tnPos)
	IF SUBSTR(m.tcJson, m.lnPos, 1) == ","
		lnPos = json_skip_space(m.tcJson, m.lnPos + 1)
	ENDIF
	RETURN m.lnPos
ENDFUNC

FUNCTION json_skip_space
	LPARAMETERS tcJson, tnPos
	LOCAL lnPos
	lnPos = m.tnPos
	DO WHILE m.lnPos <= LEN(m.tcJson) AND INLIST(SUBSTR(m.tcJson, m.lnPos, 1), " ", CHR(9), CHR(10), CHR(13))
		lnPos = m.lnPos + 1
	ENDDO
	RETURN m.lnPos
ENDFUNC

* Position after the string that opens at tnPos.
FUNCTION json_skip_string
	LPARAMETERS tcJson, tnPos
	LOCAL lnPos, lnLen, lcChar
	lnPos = m.tnPos + 1
	lnLen = LEN(m.tcJson)
	DO WHILE m.lnPos <= m.lnLen
		lcChar = SUBSTR(m.tcJson, m.lnPos, 1)
		DO CASE
		CASE m.lcChar == "\"
			lnPos = m.lnPos + 2
		CASE m.lcChar == '"'
			RETURN m.lnPos + 1
		OTHERWISE
			lnPos = m.lnPos + 1
		ENDCASE
	ENDDO
	ERROR "sunat_sdk: unterminated JSON string"
ENDFUNC

* Position after the value that starts at tnPos.
FUNCTION json_skip_value
	LPARAMETERS tcJson, tnPos
	LOCAL lcChar
	lcChar = SUBSTR(m.tcJson, m.tnPos, 1)
	DO CASE
	CASE m.lcChar == '"'
		RETURN json_skip_string(m.tcJson, m.tnPos)
	CASE INLIST(m.lcChar, "{", "[")
		RETURN json_skip_container(m.tcJson, m.tnPos)
	CASE m.tnPos > LEN(m.tcJson)
		ERROR "sunat_sdk: JSON ends where a value was expected"
	OTHERWISE
		RETURN json_skip_scalar(m.tcJson, m.tnPos)
	ENDCASE
ENDFUNC

FUNCTION json_skip_container
	LPARAMETERS tcJson, tnPos
	LOCAL lnPos, lnDepth, lcChar
	lnPos = m.tnPos
	lnDepth = 0
	DO WHILE .T.
		IF m.lnPos > LEN(m.tcJson)
			ERROR "sunat_sdk: unterminated JSON object or array"
		ENDIF
		lcChar = SUBSTR(m.tcJson, m.lnPos, 1)
		IF m.lcChar == '"'
			lnPos = json_skip_string(m.tcJson, m.lnPos)
			LOOP
		ENDIF
		lnDepth = m.lnDepth + ICASE(INLIST(m.lcChar, "{", "["), 1, INLIST(m.lcChar, "}", "]"), -1, 0)
		lnPos = m.lnPos + 1
		IF m.lnDepth = 0
			RETURN m.lnPos
		ENDIF
	ENDDO
ENDFUNC

* A number, true, false or null ends at a delimiter or at the end of the text.
FUNCTION json_skip_scalar
	LPARAMETERS tcJson, tnPos
	LOCAL lnPos
	lnPos = m.tnPos
	DO WHILE m.lnPos <= LEN(m.tcJson) AND NOT INLIST(SUBSTR(m.tcJson, m.lnPos, 1), ",", "}", "]", " ", CHR(9), CHR(10), CHR(13))
		lnPos = m.lnPos + 1
	ENDDO
	RETURN m.lnPos
ENDFUNC
