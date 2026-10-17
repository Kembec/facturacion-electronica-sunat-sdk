Attribute VB_Name = "Sample"
' Builds the test invoice of the zip, signs it with the self-generated test certificate (valid for no
' taxpayer) and shows the file name and the DigestValue, which must equal the vector's.
Option Explicit

Sub Main()
    Dim root As String, built As String, signed As String, request As String
    root = App.Path & "\.."
    On Error GoTo Failed
    SunatLoad root & "\dll\x86\sunat_sdk_ffi.dll"

    built = SunatCall("{""command"":""build"",""input"":" & ReadUtf8File(root & "\sample\01-invoice-taxed-cash.json") & "}")
    request = "{""command"":""sign""," & _
        """flags"":{""cert"":" & JsonQuote(root & "\sample\test-cert.pem") & ",""key"":" & JsonQuote(root & "\sample\test-key.pem") & "}," & _
        """input"":{""xml"":" & JsonQuote(JsonText(built, "xml")) & "}}"
    signed = SunatCall(request)

    ' Expected: 20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=
    MsgBox JsonText(built, "file_name") & " " & JsonText(signed, "digest_value"), vbInformation, "SUNAT SDK " & SunatVersion()
    Exit Sub
Failed:
    MsgBox Err.Description, vbCritical, "SUNAT SDK"
End Sub
