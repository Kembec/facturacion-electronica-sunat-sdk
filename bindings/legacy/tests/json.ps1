# The VB6 JSON reader (vb6/SunatJson.bas) over the battery's real responses, checked against
# PowerShell's own JSON parser and the vectors. The responses are the ones battery.c wrote with
# --dump through the VB6 declarations; json.vbs runs the module as VBScript and writes what it read.
#
#   pwsh -File json.ps1 -Dump <directory of --dump> -Repo <repository>
param([Parameter(Mandatory)] [string] $Dump, [Parameter(Mandatory)] [string] $Repo)
$ErrorActionPreference = 'Stop'

$utf8 = New-Object System.Text.UTF8Encoding($false)
$unicode = [Text.Encoding]::Unicode
$work = Join-Path ([IO.Path]::GetTempPath()) "sunat-sdk-json-$PID"
New-Item -ItemType Directory -Force $work | Out-Null

$cases = Get-ChildItem $Dump -Filter *.build.json | Sort-Object Name | ForEach-Object { $_.Name -replace '\.build\.json$', '' }
if ($cases.Count -eq 0) { throw "no responses in $Dump" }
# FileSystemObject reads UTF-16, not UTF-8: same text, re-encoded.
foreach ($case in $cases) {
    foreach ($kind in 'build', 'sign') {
        $text = [IO.File]::ReadAllText((Join-Path $Dump "$case.$kind.json"), $utf8)
        [IO.File]::WriteAllText((Join-Path $work "$case.$kind.json"), $text, $unicode)
    }
}

$cscript = Join-Path $env:WINDIR 'SysWOW64\cscript.exe'
if (-not (Test-Path $cscript)) { $cscript = Join-Path $env:WINDIR 'System32\cscript.exe' }
if (-not (Test-Path $cscript)) { throw 'cscript.exe (VBScript) is not on this machine' }
& $cscript //nologo (Join-Path $PSScriptRoot 'json.vbs') (Join-Path $PSScriptRoot '..\vb6\SunatJson.bas') $work @cases
if ($LASTEXITCODE -ne 0) { throw 'json.vbs failed' }

foreach ($case in $cases) {
    $built = [IO.File]::ReadAllText((Join-Path $Dump "$case.build.json"), $utf8) | ConvertFrom-Json
    $signed = [IO.File]::ReadAllText((Join-Path $Dump "$case.sign.json"), $utf8) | ConvertFrom-Json
    $xml = [IO.File]::ReadAllText((Join-Path $work "$case.xml.txt"), $unicode)
    $quoted = [IO.File]::ReadAllText((Join-Path $work "$case.quoted.txt"), $unicode)
    $values = [IO.File]::ReadAllText((Join-Path $work "$case.values.txt"), $unicode) -split "`n"
    if (-not ($xml -ceq $built.xml)) { throw "${case}: JsonText(build, ""xml"") differs from ConvertFrom-Json" }
    if (-not ((('{"v":' + $quoted + '}') | ConvertFrom-Json).v -ceq $built.xml)) { throw "${case}: JsonQuote does not give back the XML" }
    $vector = [IO.File]::ReadAllText((Join-Path $Repo "tests\vectors\$case.signed.xml"), $utf8)
    $digest = [regex]::Match($vector, '<ds:DigestValue>([^<]+)</ds:DigestValue>').Groups[1].Value
    $signature = [regex]::Match($vector, '<ds:SignatureValue>([^<]+)</ds:SignatureValue>').Groups[1].Value
    if ($values[0] -cne $built.file_name -or $values[1] -cne $digest -or $values[2] -cne $signature) {
        throw "${case}: file_name, digest_value or signature_value read by JsonText differs"
    }
}
Remove-Item -Recurse -Force $work
"ok: SunatJson.bas read $($cases.Count) build and sign responses as ConvertFrom-Json does"
