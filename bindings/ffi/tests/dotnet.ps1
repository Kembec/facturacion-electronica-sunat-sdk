# The battery from .NET Framework through P/Invoke, as a C# or VB.NET program calls the library:
# StdCall, UTF-8 bytes in, the response read with sunat_sdk_response_len and sunat_sdk_response_copy
# (the path of callers that cannot read a pointer). Run it with the 32-bit Windows PowerShell
# (SysWOW64) for the i686 DLL and the 64-bit one for the x86_64 DLL.
#
#   powershell -File dotnet.ps1 -Dll <path to sunat_sdk_ffi.dll> -Repo <repository>
param([Parameter(Mandatory)] [string] $Dll, [Parameter(Mandatory)] [string] $Repo)
$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class SunatSdk {
    [DllImport("kernel32", SetLastError = true, CharSet = CharSet.Unicode)]
    public static extern IntPtr LoadLibraryW(string path);

    [DllImport("sunat_sdk_ffi.dll", CallingConvention = CallingConvention.StdCall, ExactSpelling = true)]
    static extern int sunat_sdk_call(byte[] request, out IntPtr response);
    [DllImport("sunat_sdk_ffi.dll", CallingConvention = CallingConvention.StdCall, ExactSpelling = true)]
    static extern UIntPtr sunat_sdk_response_len(IntPtr response);
    [DllImport("sunat_sdk_ffi.dll", CallingConvention = CallingConvention.StdCall, ExactSpelling = true)]
    static extern int sunat_sdk_response_copy(IntPtr response, byte[] buffer, UIntPtr capacity);
    [DllImport("sunat_sdk_ffi.dll", CallingConvention = CallingConvention.StdCall, ExactSpelling = true)]
    static extern void sunat_sdk_free(IntPtr response);
    [DllImport("sunat_sdk_ffi.dll", CallingConvention = CallingConvention.StdCall, ExactSpelling = true)]
    public static extern uint sunat_sdk_abi_version();

    public static int Call(string request, out string response) {
        IntPtr raw;
        int status = sunat_sdk_call(Encoding.UTF8.GetBytes(request + "\0"), out raw);
        try {
            int length = checked((int)sunat_sdk_response_len(raw).ToUInt64());
            byte[] buffer = new byte[length];
            if (sunat_sdk_response_copy(raw, buffer, new UIntPtr((ulong)length)) != 0) {
                throw new InvalidOperationException("sunat_sdk_response_copy failed");
            }
            response = Encoding.UTF8.GetString(buffer);
        } finally {
            sunat_sdk_free(raw);
        }
        return status;
    }
}
'@

# Loaded by full path first, so every DllImport above binds to this module.
if ([SunatSdk]::LoadLibraryW((Resolve-Path $Dll).Path) -eq [IntPtr]::Zero) { throw "LoadLibrary failed for $Dll" }
if ([SunatSdk]::sunat_sdk_abi_version() -ne 1) { throw 'unexpected ABI version' }

function Invoke-Sdk([string] $Request, [string] $What) {
    $response = $null
    $status = [SunatSdk]::Call($Request, [ref] $response)
    if ($status -ne 0) { throw "${What}: status $status, $response" }
    return $response | ConvertFrom-Json
}

$certDir = Join-Path $Repo 'tests\vectors\test-cert'
$flags = @{ cert = (Join-Path $certDir 'test-cert.pem'); key = (Join-Path $certDir 'test-key.pem') }
$cases = Get-ChildItem (Join-Path $Repo 'fixtures') -Filter *.json | Sort-Object Name
foreach ($case in $cases) {
    $fixture = [IO.File]::ReadAllText($case.FullName, [Text.Encoding]::UTF8)
    $built = Invoke-Sdk ('{"command":"build","input":' + $fixture + '}') $case.BaseName
    $sign = @{ command = 'sign'; flags = $flags; input = @{ xml = $built.xml } } | ConvertTo-Json -Compress -Depth 5
    $signed = Invoke-Sdk $sign $case.BaseName
    $vector = [IO.File]::ReadAllText((Join-Path $Repo "tests\vectors\$($case.BaseName).signed.xml"), [Text.Encoding]::UTF8)
    $digest = [regex]::Match($vector, '<ds:DigestValue>([^<]+)</ds:DigestValue>').Groups[1].Value
    $signature = [regex]::Match($vector, '<ds:SignatureValue>([^<]+)</ds:SignatureValue>').Groups[1].Value
    if ($signed.digest_value -ne $digest -or $signed.signature_value -ne $signature) {
        throw "$($case.BaseName): DigestValue or SignatureValue differs from the vector"
    }
}
"ok: $($cases.Count) cases through P/Invoke, $([IntPtr]::Size * 8)-bit .NET $([Environment]::Version)"
