Attribute VB_Name = "SunatJson"
' A small JSON reader and writer for the SDK's requests and responses, with no references or
' components: VB6 has no JSON parser, and a COM one (ScriptControl, MSXML tricks) is one more
' thing to register on every client machine.
'
' Values are read as raw JSON text and decoded on demand, so a nested value is read in two steps:
'   JsonText(JsonGet(response, "cdr"), "response_code")
'
' The CI runs this module, as VBScript, over the 38 responses of the battery and compares it with
' PowerShell's parser (bindings/legacy/tests/json.ps1). That is why it avoids what VBScript lacks:
' the Mid$ statement, type suffixes on literals and Optional arguments.
Option Explicit

Private Const JSON_ERROR As Long = 515
Private Const HEX_DIGITS As String = "0123456789ABCDEF"

' The raw JSON text of member `key` of the object `json`, or "" when the object has no such member.
Public Function JsonGet(ByVal json As String, ByVal key As String) As String
    Dim pos As Long, keyStart As Long, valueStart As Long, valueEnd As Long
    pos = SkipSpace(json, 1)
    If Mid$(json, pos, 1) <> "{" Then Err.Raise vbObjectError + JSON_ERROR, "SunatJson", "not a JSON object"
    pos = SkipSpace(json, pos + 1)
    Do While Mid$(json, pos, 1) = """"
        keyStart = pos
        pos = SkipSpace(json, SkipString(json, pos))
        If Mid$(json, pos, 1) <> ":" Then Err.Raise vbObjectError + JSON_ERROR, "SunatJson", "expected ':' in a JSON object"
        valueStart = SkipSpace(json, pos + 1)
        valueEnd = SkipValue(json, valueStart)
        If JsonDecode(Mid$(json, keyStart, SkipString(json, keyStart) - keyStart)) = key Then
            JsonGet = Mid$(json, valueStart, valueEnd - valueStart)
            Exit Function
        End If
        pos = SkipSpace(json, valueEnd)
        If Mid$(json, pos, 1) = "," Then pos = SkipSpace(json, pos + 1)
    Loop
    JsonGet = ""
End Function

' Member `key` decoded: a string without its quotes and escapes, a number or true/false as written,
' "" for null or a missing member.
Public Function JsonText(ByVal json As String, ByVal key As String) As String
    JsonText = JsonDecode(JsonGet(json, key))
End Function

' Number of elements of the array `json`.
Public Function JsonCount(ByVal json As String) As Long
    Dim pos As Long, count As Long
    pos = SkipSpace(json, 1)
    If Mid$(json, pos, 1) <> "[" Then Err.Raise vbObjectError + JSON_ERROR, "SunatJson", "not a JSON array"
    pos = SkipSpace(json, pos + 1)
    count = 0
    Do While pos <= Len(json) And Mid$(json, pos, 1) <> "]"
        count = count + 1
        pos = SkipSpace(json, SkipValue(json, pos))
        If Mid$(json, pos, 1) = "," Then pos = SkipSpace(json, pos + 1)
    Loop
    JsonCount = count
End Function

' The raw JSON text of element `index` (from 0) of the array `json`, or "" past its end.
Public Function JsonItem(ByVal json As String, ByVal index As Long) As String
    Dim pos As Long, valueEnd As Long, current As Long
    pos = SkipSpace(json, 1)
    If Mid$(json, pos, 1) <> "[" Then Err.Raise vbObjectError + JSON_ERROR, "SunatJson", "not a JSON array"
    pos = SkipSpace(json, pos + 1)
    current = 0
    Do While pos <= Len(json) And Mid$(json, pos, 1) <> "]"
        valueEnd = SkipValue(json, pos)
        If current = index Then
            JsonItem = Mid$(json, pos, valueEnd - pos)
            Exit Function
        End If
        current = current + 1
        pos = SkipSpace(json, valueEnd)
        If Mid$(json, pos, 1) = "," Then pos = SkipSpace(json, pos + 1)
    Loop
    JsonItem = ""
End Function

' A raw JSON value as text: a string loses its quotes and escapes, null becomes "", anything else
' (number, true, false, object, array) is returned as written.
Public Function JsonDecode(ByVal token As String) As String
    Dim body As String, out As String, pos As Long, slash As Long, code As Long
    If token = "null" Then
        JsonDecode = ""
        Exit Function
    End If
    If Left$(token, 1) <> """" Then
        JsonDecode = token
        Exit Function
    End If
    body = Mid$(token, 2, Len(token) - 2)
    out = ""
    pos = 1
    Do
        slash = InStr(pos, body, "\")
        If slash = 0 Then Exit Do
        out = out & Mid$(body, pos, slash - pos)
        Select Case Mid$(body, slash + 1, 1)
            Case "n": out = out & vbLf
            Case "r": out = out & vbCr
            Case "t": out = out & vbTab
            Case "b": out = out & ChrW$(8)
            Case "f": out = out & ChrW$(12)
            Case "u"
                code = HexValue(Mid$(body, slash + 2, 4))
                out = out & ChrW$(code)
                slash = slash + 4
            Case Else: out = out & Mid$(body, slash + 1, 1)
        End Select
        pos = slash + 2
    Loop
    JsonDecode = out & Mid$(body, pos)
End Function

' `text` as a JSON string literal, quotes included. Characters outside ASCII stay as they are: the
' request goes to the library as UTF-8.
Public Function JsonQuote(ByVal text As String) As String
    Dim out As String, pos As Long, start As Long, code As Long
    out = """"
    start = 1
    For pos = 1 To Len(text)
        code = AscW(Mid$(text, pos, 1))
        If code = 34 Or code = 92 Or (code >= 0 And code < 32) Then
            out = out & Mid$(text, start, pos - start) & EscapeOf(code)
            start = pos + 1
        End If
    Next
    JsonQuote = out & Mid$(text, start) & """"
End Function

Private Function EscapeOf(ByVal code As Long) As String
    Select Case code
        Case 34: EscapeOf = "\"""
        Case 92: EscapeOf = "\\"
        Case 10: EscapeOf = "\n"
        Case 13: EscapeOf = "\r"
        Case 9: EscapeOf = "\t"
        Case Else: EscapeOf = "\u00" & Mid$(HEX_DIGITS, code \ 16 + 1, 1) & Mid$(HEX_DIGITS, code Mod 16 + 1, 1)
    End Select
End Function

' Four hexadecimal digits as a number from 0 to 65535. Not CLng("&H..."): VB6 reads "&HFFFF" as
' the Integer -1.
Private Function HexValue(ByVal digits As String) As Long
    Dim pos As Long, digit As Long, value As Long
    If Len(digits) <> 4 Then Err.Raise vbObjectError + JSON_ERROR, "SunatJson", "bad \u escape"
    value = 0
    For pos = 1 To 4
        digit = InStr(HEX_DIGITS, UCase$(Mid$(digits, pos, 1))) - 1
        If digit < 0 Then Err.Raise vbObjectError + JSON_ERROR, "SunatJson", "bad \u escape"
        value = value * 16 + digit
    Next
    HexValue = value
End Function

Private Function SkipSpace(ByVal json As String, ByVal pos As Long) As Long
    Do While pos <= Len(json)
        If InStr(" " & vbTab & vbCr & vbLf, Mid$(json, pos, 1)) = 0 Then Exit Do
        pos = pos + 1
    Loop
    SkipSpace = pos
End Function

' Position after the string that opens at `pos`.
Private Function SkipString(ByVal json As String, ByVal pos As Long) As Long
    Dim quote As Long, slash As Long
    pos = pos + 1
    Do
        quote = InStr(pos, json, """")
        If quote = 0 Then Err.Raise vbObjectError + JSON_ERROR, "SunatJson", "unterminated JSON string"
        slash = InStr(pos, json, "\")
        If slash = 0 Or slash > quote Then Exit Do
        pos = slash + 2
    Loop
    SkipString = quote + 1
End Function

' Position after the value that starts at `pos`.
Private Function SkipValue(ByVal json As String, ByVal pos As Long) As Long
    Select Case Mid$(json, pos, 1)
        Case """": SkipValue = SkipString(json, pos)
        Case "{", "[": SkipValue = SkipContainer(json, pos)
        Case "": Err.Raise vbObjectError + JSON_ERROR, "SunatJson", "JSON ends where a value was expected"
        Case Else: SkipValue = SkipScalar(json, pos)
    End Select
End Function

Private Function SkipContainer(ByVal json As String, ByVal pos As Long) As Long
    Dim depth As Long, c As String
    depth = 0
    Do
        c = Mid$(json, pos, 1)
        If c = "" Then Err.Raise vbObjectError + JSON_ERROR, "SunatJson", "unterminated JSON object or array"
        If c = """" Then
            pos = SkipString(json, pos)
        Else
            If c = "{" Or c = "[" Then depth = depth + 1
            If c = "}" Or c = "]" Then depth = depth - 1
            pos = pos + 1
        End If
    Loop Until depth = 0
    SkipContainer = pos
End Function

' A number, true, false or null ends at a delimiter or at the end of the text.
Private Function SkipScalar(ByVal json As String, ByVal pos As Long) As Long
    Do While pos <= Len(json)
        If InStr(",}] " & vbTab & vbCr & vbLf, Mid$(json, pos, 1)) > 0 Then Exit Do
        pos = pos + 1
    Loop
    SkipScalar = pos
End Function
