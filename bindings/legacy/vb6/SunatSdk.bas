Attribute VB_Name = "SunatSdk"
' SUNAT electronic invoicing SDK for VB6, through the C ABI of sunat_sdk_ffi.dll (docs/c-abi.md).
'
'   SunatLoad App.Path & "\sunat_sdk_ffi.dll"          ' once, before any call
'   built = SunatCall("{""command"":""build"",""input"":" & documentJson & "}")
'   xml = JsonText(built, "xml")                        ' SunatJson.bas
'
' Text crosses the DLL as UTF-8 bytes in a Byte array, never As String: a String argument of a
' Declare is converted to the ANSI code page, which loses every character outside it. The response
' is copied into a Byte array with sunat_sdk_response_len and sunat_sdk_response_copy, because VB6
' cannot read through a pointer, and released with sunat_sdk_free right after.
'
' bindings/legacy/tests/declarations.py reads the six Declare lines below and the CI calls the
' 32-bit DLL through exactly these types; keep each Declare on one line.
Option Explicit

' The DLL is __stdcall with undecorated names: no Alias.
Private Declare Function sunat_sdk_call Lib "sunat_sdk_ffi.dll" (ByRef request As Byte, ByRef response As Long) As Long
Private Declare Function sunat_sdk_response_len Lib "sunat_sdk_ffi.dll" (ByVal response As Long) As Long
Private Declare Function sunat_sdk_response_copy Lib "sunat_sdk_ffi.dll" (ByVal response As Long, ByRef buffer As Byte, ByVal capacity As Long) As Long
Private Declare Sub sunat_sdk_free Lib "sunat_sdk_ffi.dll" (ByVal response As Long)
Private Declare Function sunat_sdk_version Lib "sunat_sdk_ffi.dll" () As Long
Private Declare Function sunat_sdk_abi_version Lib "sunat_sdk_ffi.dll" () As Long

Private Declare Function LoadLibraryW Lib "kernel32" (ByVal path As Long) As Long
Private Declare Function WideCharToMultiByte Lib "kernel32" (ByVal codePage As Long, ByVal flags As Long, ByVal wide As Long, ByVal wideLength As Long, ByVal multi As Long, ByVal multiLength As Long, ByVal defaultChar As Long, ByVal usedDefault As Long) As Long
Private Declare Function MultiByteToWideChar Lib "kernel32" (ByVal codePage As Long, ByVal flags As Long, ByVal multi As Long, ByVal multiLength As Long, ByVal wide As Long, ByVal wideLength As Long) As Long
Private Declare Function lstrlenA Lib "kernel32" (ByVal text As Long) As Long
Private Declare Sub CopyMemory Lib "kernel32" Alias "RtlMoveMemory" (ByVal destination As Long, ByVal source As Long, ByVal length As Long)

Private Const CP_UTF8 As Long = 65001
Private Const MB_ERR_INVALID_CHARS As Long = 8
Private Const WC_ERR_INVALID_CHARS As Long = 128
Private Const SUNAT_SDK_OK As Long = 0
Private Const SUNAT_SDK_ABI_VERSION As Long = 1

' Err.Number of every error raised here: vbObjectError + SUNAT_SDK_FAILED. Err.Description is the
' SDK's message (the CLI's "error"), which never carries a password or a secret.
Public Const SUNAT_SDK_FAILED As Long = 513

' Loads the DLL from a full path and checks its ABI. The Declare statements then bind to this
' module by name, so the DLL does not have to be next to VB6.EXE or on the PATH while debugging.
Public Sub SunatLoad(ByVal dllPath As String)
    If LoadLibraryW(StrPtr(dllPath)) = 0 Then
        Err.Raise vbObjectError + SUNAT_SDK_FAILED, "SunatSdk", _
            "cannot load " & dllPath & " (error " & Err.LastDllError & "); VB6 needs the 32-bit DLL, dll\x86"
    End If
    If sunat_sdk_abi_version() <> SUNAT_SDK_ABI_VERSION Then
        Err.Raise vbObjectError + SUNAT_SDK_FAILED, "SunatSdk", "sunat_sdk_ffi.dll is of another ABI version"
    End If
End Sub

' The SDK version of the loaded DLL, for example "0.1.0-beta.5".
Public Function SunatVersion() As String
    Dim pointer As Long, length As Long, bytes() As Byte
    pointer = sunat_sdk_version()
    length = lstrlenA(pointer)
    ReDim bytes(0 To length) As Byte
    CopyMemory VarPtr(bytes(0)), pointer, length
    SunatVersion = FromUtf8(bytes, length)
End Function

' Runs one command (docs/c-abi.md) and returns its JSON response. A rejected command raises
' vbObjectError + SUNAT_SDK_FAILED with the SDK's message. A SOAP fault or a query without verdict is
' not an error: it is the "outcome" of the response, as in the CLI.
Public Function SunatCall(ByVal requestJson As String) As String
    Dim request() As Byte, response As Long, status As Long, text As String
    request = ToUtf8(requestJson)
    status = sunat_sdk_call(request(0), response)
    If response = 0 Then Err.Raise vbObjectError + SUNAT_SDK_FAILED, "SunatSdk", "sunat_sdk_call gave no response"
    text = TakeResponse(response)
    If status <> SUNAT_SDK_OK Then Err.Raise vbObjectError + SUNAT_SDK_FAILED, "SunatSdk", JsonText(text, "error")
    SunatCall = text
End Function

' Copies the response out of the library and releases it, whatever happens.
Private Function TakeResponse(ByVal response As Long) As String
    Dim length As Long, buffer() As Byte, copied As Long
    length = sunat_sdk_response_len(response)
    ReDim buffer(0 To length) As Byte
    copied = sunat_sdk_response_copy(response, buffer(0), length + 1)
    sunat_sdk_free response
    If copied <> SUNAT_SDK_OK Then Err.Raise vbObjectError + SUNAT_SDK_FAILED, "SunatSdk", "sunat_sdk_response_copy failed"
    TakeResponse = FromUtf8(buffer, length)
End Function

' `text` as UTF-8 bytes followed by a NUL, as sunat_sdk_call reads it.
Private Function ToUtf8(ByVal text As String) As Byte()
    Dim size As Long, bytes() As Byte
    ReDim bytes(0) As Byte
    If Len(text) > 0 Then
        size = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, StrPtr(text), Len(text), 0, 0, 0, 0)
        If size = 0 Then Err.Raise vbObjectError + SUNAT_SDK_FAILED, "SunatSdk", "the request is not valid Unicode"
        ReDim bytes(0 To size) As Byte
        WideCharToMultiByte CP_UTF8, WC_ERR_INVALID_CHARS, StrPtr(text), Len(text), VarPtr(bytes(0)), size, 0, 0
    End If
    ToUtf8 = bytes
End Function

Private Function FromUtf8(bytes() As Byte, ByVal length As Long) As String
    Dim size As Long, text As String
    If length = 0 Then Exit Function
    size = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, VarPtr(bytes(0)), length, 0, 0)
    If size = 0 Then Err.Raise vbObjectError + SUNAT_SDK_FAILED, "SunatSdk", "the response is not UTF-8"
    text = String$(size, vbNullChar)
    MultiByteToWideChar CP_UTF8, MB_ERR_INVALID_CHARS, VarPtr(bytes(0)), length, StrPtr(text), size
    FromUtf8 = text
End Function

' Reads a UTF-8 file (a document, a certificate path list) into a String, without its BOM if it has
' one. VB6's Open ... For Input reads the ANSI code page instead.
Public Function ReadUtf8File(ByVal path As String) As String
    Dim file As Integer, bytes() As Byte, length As Long, text As String
    file = FreeFile
    Open path For Binary Access Read As #file
    length = LOF(file)
    If length = 0 Then
        Close #file
        Exit Function
    End If
    ReDim bytes(0 To length - 1) As Byte
    Get #file, , bytes
    Close #file
    text = FromUtf8(bytes, length)
    If Left$(text, 1) = ChrW$(&HFEFF) Then text = Mid$(text, 2)
    ReadUtf8File = text
End Function
