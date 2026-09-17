[CmdletBinding()]
param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\omg",
    [string]$DownloadBaseUrl = "https://github.com/Y-Square-T3/oh-my-gocd/releases/latest/download",
    [switch]$SkipPathUpdate,
    [switch]$SkipVersionCheck
)

$ErrorActionPreference = "Stop"
$archiveName = "omg-windows-amd64.zip"

if (-not $IsWindows -and $PSVersionTable.PSEdition -eq "Core") {
    throw "This installer supports Windows only."
}

$architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
if ($architecture -ne [System.Runtime.InteropServices.Architecture]::X64) {
    throw "No prebuilt binary is available for architecture $architecture. Use cargo install oh-my-gocd."
}

function Copy-Download {
    param(
        [string]$Source,
        [string]$Destination
    )

    if ($Source -match '^https?://') {
        Invoke-WebRequest -Uri $Source -OutFile $Destination -UseBasicParsing
    } else {
        Copy-Item -LiteralPath $Source -Destination $Destination
    }
}

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("omg-install-" + [guid]::NewGuid())
try {
    New-Item -ItemType Directory -Path $tempDir | Out-Null
    $archivePath = Join-Path $tempDir $archiveName
    $checksumsPath = Join-Path $tempDir "SHA256SUMS"

    if ($DownloadBaseUrl -match '^https?://') {
        $archiveSource = "$DownloadBaseUrl/$archiveName"
        $checksumsSource = "$DownloadBaseUrl/SHA256SUMS"
    } else {
        $archiveSource = Join-Path $DownloadBaseUrl $archiveName
        $checksumsSource = Join-Path $DownloadBaseUrl "SHA256SUMS"
    }

    Copy-Download -Source $archiveSource -Destination $archivePath
    Copy-Download -Source $checksumsSource -Destination $checksumsPath

    $checksumLine = Get-Content $checksumsPath | Where-Object { $_ -match "^[a-fA-F0-9]{64}\s+\*?$([regex]::Escape($archiveName))$" } | Select-Object -First 1
    if (-not $checksumLine) {
        throw "SHA256SUMS does not contain $archiveName."
    }

    $expectedHash = ($checksumLine -split '\s+')[0]
    $actualHash = (Get-FileHash -Algorithm SHA256 $archivePath).Hash
    if ($actualHash -ne $expectedHash) {
        throw "Checksum verification failed for $archiveName."
    }

    $extractDir = Join-Path $tempDir "extracted"
    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractDir
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item -LiteralPath (Join-Path $extractDir "omg.exe") -Destination (Join-Path $InstallDir "omg.exe") -Force

    if (-not $SkipPathUpdate) {
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        $pathEntries = @($userPath -split ';' | Where-Object { $_ })
        if ($pathEntries -notcontains $InstallDir) {
            $newUserPath = (@($InstallDir) + $pathEntries) -join ';'
            [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
            Write-Host "Added $InstallDir to your user PATH."
        }
        if (($env:Path -split ';') -notcontains $InstallDir) {
            $env:Path = "$InstallDir;$env:Path"
        }
    }

    Write-Host "Installed omg to $(Join-Path $InstallDir 'omg.exe')"
    if (-not $SkipVersionCheck) {
        & (Join-Path $InstallDir "omg.exe") --version
    }
} finally {
    Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
}
