Option Strict On

Imports System.IO
Imports System.Text
Imports System.Text.RegularExpressions
Imports Kembec.SunatSdk

' The project's RootNamespace (Kembec.SunatSdk.Vectors) is this module's namespace.
''' <summary>
''' The 38 fixtures from VB.NET with Option Strict: each signed DigestValue and SignatureValue equal to its
''' vector's. FirstVoucher is the README's example as written there.
''' </summary>
Module Program

    Function Main(args As String()) As Integer
        If args.Length <> 1 Then
            Console.Error.WriteLine("usage: Vectors.VisualBasic <repository>")
            Return 2
        End If
        Dim repo = Path.GetFullPath(args(0))
        Dim certDir = Path.Combine(repo, "tests", "vectors", "test-cert")
        Dim signingKey As New SigningKey(Path.Combine(certDir, "test-cert.pem"), Path.Combine(certDir, "test-key.pem"))
        Dim client As New SunatSdkClient(TimeSpan.FromSeconds(30))
        Dim failures = 0

        Dim fixtures = Directory.GetFiles(Path.Combine(repo, "fixtures"), "*.json")
        Array.Sort(fixtures, StringComparer.Ordinal)
        If fixtures.Length <> 38 Then
            Console.Error.WriteLine("FAIL fixtures: " & fixtures.Length & ", expected 38")
            failures += 1
        End If

        For Each fixture In fixtures
            Dim name = Path.GetFileNameWithoutExtension(fixture)
            Try
                Dim built = client.Build(File.ReadAllText(fixture, Encoding.UTF8))
                Dim signed = client.Sign(built.Xml, signingKey)
                Dim vector = File.ReadAllText(Path.Combine(repo, "tests", "vectors", name & ".signed.xml"), Encoding.UTF8)
                Dim digest = Regex.Match(vector, "<ds:DigestValue>([^<]+)</ds:DigestValue>").Groups(1).Value
                Dim signature = Regex.Match(vector, "<ds:SignatureValue>([^<]+)</ds:SignatureValue>").Groups(1).Value
                If signed.DigestValue <> digest OrElse signed.SignatureValue <> signature Then
                    Console.Error.WriteLine("FAIL " & name & ": DigestValue or SignatureValue differs from the vector")
                    failures += 1
                End If
            Catch e As SunatSdkException
                Console.Error.WriteLine("FAIL " & name & ": " & e.Message)
                failures += 1
            End Try
        Next

        Dim example = FirstVoucher(repo)
        If example <> "20123456789-01-F001-1 +te61a2ub5MlTFf8tKTPNGWEXKI=" Then
            Console.Error.WriteLine("FAIL README example printed " & example)
            failures += 1
        End If

        Dim runtime = (IntPtr.Size * 8).ToString() & "-bit " & System.Runtime.InteropServices.RuntimeInformation.FrameworkDescription
        If failures > 0 Then
            Console.Error.WriteLine("VB.NET: " & failures & " failures (" & runtime & ")")
            Return 1
        End If
        Console.WriteLine("ok VB.NET: " & fixtures.Length & " vectors, " & runtime)
        Return 0
    End Function

    ''' <summary>bindings/dotnet/README.md, «Tu primer comprobante», VB.NET.</summary>
    Private Function FirstVoucher(repo As String) As String
        Dim sdk As New SunatSdkClient()
        Dim built = sdk.Build(File.ReadAllText(Path.Combine(repo, "fixtures", "01-invoice-taxed-cash.json")))
        Dim certDir = Path.Combine(repo, "tests", "vectors", "test-cert")
        Dim signed = sdk.Sign(built.Xml, New SigningKey(Path.Combine(certDir, "test-cert.pem"), Path.Combine(certDir, "test-key.pem")))
        Return built.FileName & " " & signed.DigestValue
    End Function

End Module
