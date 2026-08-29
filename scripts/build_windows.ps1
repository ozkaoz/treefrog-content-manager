#Requires -Version 5.1
<#
.SYNOPSIS
    Reproducible Windows x64 build for TreeFrog Content Manager (Tauri 2)
.DESCRIPTION
    Builds the TreeFrog Content Manager desktop application for Windows x64.
    Prerequisites: Rust (stable), Node.js 18+, Tauri CLI, FFmpeg/ffprobe (for video pipeline runtime, not required for build)
    Output: treefrog-manager/src-tauri/target/release/treefrog-manager.exe and bundle installers
.NOTES
    Run from repository root: powershell -ExecutionPolicy Bypass -File scripts\build_windows.ps1
    Or: npm run tauri build  (after npm install)
#>
param(
    [switch]$NoBundle = $false,
    [switch]$Verbose = $false
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$ManagerDir = Join-Path $RepoRoot "treefrog-manager"

Write-Host "=== TreeFrog Content Manager — Windows x64 Build ===" -ForegroundColor Cyan
Write-Host "Repo: $RepoRoot"
Write-Host "Manager: $ManagerDir"

# Check prerequisites
function Test-Command($cmd) {
    try { Get-Command $cmd -ErrorAction Stop | Out-Null; return $true } catch { return $false }
}

$missing = @()
if (-not (Test-Command "cargo")) { $missing += "Rust (cargo) — install via https://rustup.rs (stable)" }
if (-not (Test-Command "node")) { $missing += "Node.js 18+ — https://nodejs.org" }
if (-not (Test-Command "npm")) { $missing += "npm (comes with Node.js)" }
if (-not (Test-Command "ffmpeg")) { Write-Host "WARNING: ffmpeg not found — video pipeline requires ffmpeg/ffprobe at runtime, but build can proceed" -ForegroundColor Yellow }
if (-not (Test-Command "ffprobe")) { Write-Host "WARNING: ffprobe not found — video inspection requires ffprobe at runtime" -ForegroundColor Yellow }

if ($missing.Count -gt 0) {
    Write-Host "Missing prerequisites:" -ForegroundColor Red
    $missing | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    Write-Host "`nInstall missing tools and re-run. For Rust: https://rustup.rs" -ForegroundColor Yellow
    Write-Host "For Node.js: https://nodejs.org (use 18+)" -ForegroundColor Yellow
    exit 1
}

Write-Host "`nPrerequisites:"
Write-Host "  cargo: $(cargo --version)"
Write-Host "  rustc: $(rustc --version)"
Write-Host "  node: $(node --version)"
Write-Host "  npm: $(npm --version)"
try { Write-Host "  ffmpeg: $(ffmpeg -version 2>&1 | Select-Object -First 1)" } catch {}
try { Write-Host "  ffprobe: $(ffprobe -version 2>&1 | Select-Object -First 1)" } catch {}

# Ensure Tauri CLI
if (-not (Test-Command "cargo-tauri") -and -not (Test-Path "$ManagerDir\node_modules\.bin\tauri.cmd")) {
    Write-Host "`nInstalling Tauri CLI..." -ForegroundColor Yellow
    Push-Location $ManagerDir
    npm install
    Pop-Location
}

Write-Host "`n=== Installing frontend dependencies ===" -ForegroundColor Cyan
Push-Location $ManagerDir
if (-not (Test-Path "node_modules")) {
    npm install
    if ($LASTEXITCODE -ne 0) { throw "npm install failed" }
} else {
    Write-Host "node_modules already exists, skipping npm install (run npm install manually if needed)"
}

Write-Host "`n=== Building frontend (vite) ===" -ForegroundColor Cyan
npm run build
if ($LASTEXITCODE -ne 0) { throw "npm run build failed" }

Write-Host "`n=== Building Tauri Windows x64 ===" -ForegroundColor Cyan
# Use Tauri CLI via npx or cargo
$tauriCmd = if (Test-Path "node_modules\.bin\tauri.cmd") { "npx tauri build" } else { "cargo tauri build" }
if ($NoBundle) {
    $tauriCmd += " --no-bundle"
}
if ($Verbose) {
    $tauriCmd += " --verbose"
}

# Set target to x86_64-pc-windows-msvc (default on Windows)
Write-Host "Running: $tauriCmd" -ForegroundColor Gray
Invoke-Expression $tauriCmd
if ($LASTEXITCODE -ne 0) { throw "Tauri build failed" }

Pop-Location

Write-Host "`n=== Build artifacts ===" -ForegroundColor Green
$exe = Join-Path $ManagerDir "src-tauri\target\release\treefrog-manager.exe"
$bundleMsi = Join-Path $ManagerDir "src-tauri\target\release\bundle\msi\*.msi"
$bundleNsis = Join-Path $ManagerDir "src-tauri\target\release\bundle\nsis\*.exe"

if (Test-Path $exe) {
    $info = Get-Item $exe
    Write-Host "EXE: $exe ($([math]::Round($info.Length/1MB,2)) MB)" -ForegroundColor Green
} else {
    Write-Host "WARNING: EXE not found at $exe" -ForegroundColor Yellow
}

Get-ChildItem (Join-Path $ManagerDir "src-tauri\target\release\bundle") -Recurse -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host "Bundle: $($_.FullName) ($([math]::Round($_.Length/1MB,2)) MB)" -ForegroundColor Green
}

# --- Windows installer Desktop workflow (LGPT milestone) ---
Write-Host "`n=== Desktop installer workflow ===" -ForegroundColor Cyan
$nsisDir = Join-Path $ManagerDir "src-tauri\target\release\bundle\nsis"
$nsisInstaller = Get-ChildItem -Path $nsisDir -Filter "*.exe" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if ($nsisInstaller) {
    Write-Host "Found NSIS installer: $($nsisInstaller.FullName) ($([math]::Round($nsisInstaller.Length/1MB,2)) MB)" -ForegroundColor Green
    # Resolve Desktop path reliably (handles OneDrive redirection)
    $desktopPath = [Environment]::GetFolderPath("Desktop")
    if (-not $desktopPath -or -not (Test-Path $desktopPath)) {
        # Fallback via Shell
        try {
            $shell = New-Object -ComObject WScript.Shell
            $desktopPath = $shell.SpecialFolders.Item("Desktop")
        } catch {}
    }
    if (-not $desktopPath) {
        $desktopPath = Join-Path $env:USERPROFILE "Desktop"
    }
    Write-Host "Desktop resolved to: $desktopPath" -ForegroundColor Gray
    if (Test-Path $desktopPath) {
        $friendlyName = "TreeFrog-Content-Manager-Setup.exe"
        $dest = Join-Path $desktopPath $friendlyName
        $checksumDest = "$dest.sha256"
        Write-Host "Copying installer to Desktop as $friendlyName ..." -ForegroundColor Cyan
        try {
            Copy-Item -LiteralPath $nsisInstaller.FullName -Destination $dest -Force
            Write-Host "Copied to: $dest" -ForegroundColor Green
            # Create SHA-256 checksum file
            $hash = (Get-FileHash -LiteralPath $nsisInstaller.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            $checksumLine = "$hash  $friendlyName"
            Set-Content -LiteralPath $checksumDest -Value $checksumLine -Encoding ascii
            Write-Host "Checksum created: $checksumDest" -ForegroundColor Green
            Write-Host "SHA-256: $hash" -ForegroundColor Gray
            Write-Host "Original bundle remains at: $($nsisInstaller.FullName)" -ForegroundColor Gray
        } catch {
            Write-Host "Failed to copy installer to Desktop: $_" -ForegroundColor Red
        }
    } else {
        Write-Host "WARNING: Could not resolve Desktop path, skipping copy. Installer remains at $($nsisInstaller.FullName)" -ForegroundColor Yellow
    }
} else {
    Write-Host "WARNING: No NSIS installer found in $nsisDir (build may have been --no-bundle or failed)" -ForegroundColor Yellow
}

Write-Host "`n=== Smoke test ===" -ForegroundColor Cyan
Write-Host "To smoke test, run: & `"$exe`" --self-check"
Write-Host "Or launch the GUI: & `"$exe`""
Write-Host "The app should: load TreeFrogUI profile, allow source selection, generate dry-run without SD writes."

Write-Host "`nBuild complete." -ForegroundColor Green
