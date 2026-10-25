# Installs the comtrya binary from GitHub releases. Served as https://comtrya.dev/install.ps1
#
#   irm https://comtrya.dev/install.ps1 | iex
#
# Environment variables:
#   VERSION      release tag to install, e.g. v0.10.0 (default: the latest release)
#   INSTALL_DIR  where to put comtrya.exe (default: $env:USERPROFILE\.comtrya)
#   GITHUB_TOKEN optional, avoids GitHub API rate limits (e.g. in CI)
$ErrorActionPreference = "Stop"

$Repo = "comtrya/comtrya"
$Asset = "comtrya-x86_64-pc-windows-msvc.exe"
$InstallDir = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { Join-Path $env:USERPROFILE ".comtrya" }

$Api = if ($env:VERSION) {
    "https://api.github.com/repos/$Repo/releases/tags/$env:VERSION"
} else {
    "https://api.github.com/repos/$Repo/releases/latest"
}
$Headers = @{ Accept = "application/vnd.github+json" }
if ($env:GITHUB_TOKEN) {
    $Headers.Authorization = "Bearer $env:GITHUB_TOKEN"
}
$Release = Invoke-RestMethod -Uri $Api -Headers $Headers
$Download = $Release.assets | Where-Object name -EQ $Asset
if (-not $Download) {
    throw "Release $($Release.tag_name) has no $Asset"
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$Target = Join-Path $InstallDir "comtrya.exe"
$Temp = "$Target.download"

Write-Host "Downloading comtrya $($Release.tag_name)"
Invoke-WebRequest -Uri $Download.browser_download_url -OutFile $Temp -UseBasicParsing

# The SHA-256 GitHub recorded for the asset. Releases before mid-2025 have none
if ($Download.digest) {
    $Expected = $Download.digest -replace "^sha256:", ""
    $Actual = (Get-FileHash -Algorithm SHA256 $Temp).Hash.ToLower()
    if ($Actual -ne $Expected) {
        Remove-Item $Temp
        throw "Checksum mismatch: expected $Expected, got $Actual"
    }
    Write-Host "Checksum verified"
} else {
    Write-Warning "Release $($Release.tag_name) has no published checksum, skipping verification"
}

Move-Item -Force $Temp $Target
Write-Host "Installed $(& $Target --version) to $Target"

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not (($UserPath -split ";") -contains $InstallDir)) {
    $NewPath = if ($UserPath) { "$UserPath;$InstallDir" } else { $InstallDir }
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    Write-Host "Added $InstallDir to your PATH. Open a new terminal to use comtrya."
}
