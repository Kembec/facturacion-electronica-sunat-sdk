' Runs vb6/SunatJson.bas as VBScript, the only VB that runs in CI.
'
'   cscript //nologo json.vbs <SunatJson.bas> <directory> <case>...
'
' The module is loaded with its type declarations removed (VBScript has only Variants). For every
' case it reads <case>.build.json and <case>.sign.json (UTF-16, written by json.ps1) and writes, also
' in UTF-16, what the module reads from them: <case>.xml.txt (the XML), <case>.quoted.txt (that XML
' through JsonQuote) and <case>.values.txt (file name, DigestValue, SignatureValue, one per line).
' json.ps1 compares those with PowerShell's own JSON parser and the vectors. Fixed cases first.
Option Explicit

Dim fso, args, failures
Set fso = CreateObject("Scripting.FileSystemObject")
Set args = WScript.Arguments
failures = 0

ExecuteGlobal AsVBScript(ReadAscii(args(0)))

Sub Check(ByVal what, ByVal actual, ByVal expected)
    If actual <> expected Then
        WScript.StdErr.WriteLine "FAIL " & what & ": got [" & actual & "], expected [" & expected & "]"
        failures = failures + 1
    End If
End Sub

Sub CheckRaises(ByVal what, ByVal json, ByVal key)
    Dim value
    On Error Resume Next
    value = JsonGet(json, key)
    If Err.Number <> vbObjectError + 515 Then
        WScript.StdErr.WriteLine "FAIL " & what & ": no error raised"
        failures = failures + 1
    End If
    On Error GoTo 0
End Sub

Function ReadAscii(ByVal path)
    ReadAscii = fso.OpenTextFile(path, 1, False, 0).ReadAll()
End Function

Function ReadUnicode(ByVal path)
    ReadUnicode = fso.OpenTextFile(path, 1, False, -1).ReadAll()
End Function

Sub WriteUnicode(ByVal path, ByVal text)
    Dim file
    Set file = fso.CreateTextFile(path, True, True)
    file.Write text
    file.Close
End Sub

' VB6 to VBScript: no Attribute or Option lines, no "As Type", no "$" on string functions.
Function AsVBScript(ByVal source)
    Dim re
    Set re = New RegExp
    re.Global = True
    re.IgnoreCase = True
    re.Multiline = True
    re.Pattern = "^(Attribute|Option) .*$"
    source = re.Replace(source, "")
    re.Pattern = " As (Long|String|Integer|Boolean|Byte|Variant)(\(\))?"
    source = re.Replace(source, "")
    re.Pattern = "\b(Mid|Left|Right|Space|ChrW|Chr|UCase|LCase|String)\$"
    AsVBScript = re.Replace(source, "$1")
End Function

Dim cdr, notes, index, name, folder, built, signed, xml
Check "escapes", JsonText("{""a"":""x\u00f1\ud83d\ude00\n\""\\\/\t""}", "a"), _
    "x" & ChrW(241) & ChrW(55357) & ChrW(56832) & vbLf & """" & "\" & "/" & vbTab
Check "quote", JsonQuote(ChrW(1) & "a""b\c" & vbCrLf & vbTab & ChrW(241)), """\u0001a\""b\\c\r\n\t" & ChrW(241) & """"
Check "number", JsonText("{ ""a"" : 12.5 , ""b"":true}", "a"), "12.5"
Check "true", JsonText("{ ""a"" : 12.5 , ""b"":true}", "b"), "true"
Check "null", JsonText("{""a"":null}", "a"), ""
Check "missing", JsonGet("{""a"":1}", "b"), ""
Check "brace in a string", JsonText("{ ""a"" : [ 1 , { ""b"" : ""}]\"""" } ] , ""c"" : ""d"" }", "c"), "d"
Check "escaped key", JsonText("{""a\""b"":1,""a"":2}", "a"), "2"
cdr = JsonGet("{""outcome"":""cdr"",""cdr"":{""code"":""0"",""notes"":[""4001 a"",""4002 b""]}}", "cdr")
Check "nested", JsonText(cdr, "code"), "0"
notes = JsonGet(cdr, "notes")
Check "count", JsonCount(notes), 2
Check "count empty", JsonCount("[ ]"), 0
Check "item", JsonDecode(JsonItem(notes, 1)), "4002 b"
Check "item past the end", JsonItem(notes, 2), ""
CheckRaises "not an object", "[1]", "a"
CheckRaises "unterminated", "{""a"":""x", "a"

folder = args(1)
For index = 2 To args.Count - 1
    name = args(index)
    built = ReadUnicode(folder & "\" & name & ".build.json")
    signed = ReadUnicode(folder & "\" & name & ".sign.json")
    xml = JsonText(built, "xml")
    WriteUnicode folder & "\" & name & ".xml.txt", xml
    WriteUnicode folder & "\" & name & ".quoted.txt", JsonQuote(xml)
    WriteUnicode folder & "\" & name & ".values.txt", JsonText(built, "file_name") & vbLf & _
        JsonText(signed, "digest_value") & vbLf & JsonText(signed, "signature_value")
Next

If failures > 0 Then WScript.Quit 1
WScript.Echo "ok: SunatJson.bas, fixed cases and " & (args.Count - 2) & " responses"
