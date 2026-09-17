$ErrorActionPreference = "Stop"

$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("omg-install-test-" + [guid]::NewGuid())
$releaseDir = Join-Path $testRoot "release"
$packageDir = Join-Path $testRoot "package"
$installDir = Join-Path $testRoot "install"

try {
    function global:cargo { throw "installer invoked cargo" }

    New-Item -ItemType Directory -Path $releaseDir, $packageDir | Out-Null
    Set-Content -Path (Join-Path $packageDir "omg.exe") -Value "test binary"
    Compress-Archive -Path (Join-Path $packageDir "omg.exe") -DestinationPath (Join-Path $releaseDir "omg-windows-amd64.zip")

    $hash = (Get-FileHash -Algorithm SHA256 (Join-Path $releaseDir "omg-windows-amd64.zip")).Hash.ToLowerInvariant()
    Set-Content -Path (Join-Path $releaseDir "SHA256SUMS") -Value "$hash  omg-windows-amd64.zip"

    & "$PSScriptRoot/../install.ps1" -DownloadBaseUrl $releaseDir -InstallDir $installDir -SkipPathUpdate -SkipVersionCheck
    if (-not (Test-Path (Join-Path $installDir "omg.exe"))) {
        throw "installer did not install omg.exe"
    }

    Set-Content -Path (Join-Path $releaseDir "SHA256SUMS") -Value "0000000000000000000000000000000000000000000000000000000000000000  omg-windows-amd64.zip"
    try {
        & "$PSScriptRoot/../install.ps1" -DownloadBaseUrl $releaseDir -InstallDir $installDir -SkipPathUpdate -SkipVersionCheck
        throw "installer accepted an invalid checksum"
    } catch {
        if ($_.Exception.Message -eq "installer accepted an invalid checksum") {
            throw
        }
    }

    Write-Host "Windows installer tests passed"
} finally {
    Remove-Item Function:\cargo -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force $testRoot -ErrorAction SilentlyContinue
}
