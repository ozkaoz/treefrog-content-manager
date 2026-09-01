# quick_validate.ps1 - Minimal local validation per change class (DEC-2026-09-01-02)
# Detects what changed vs origin/main and runs ONLY the relevant checks.
# The FULL matrix runs automatically in CI (validate.yml) on every push.
#
# Usage:
#   powershell -File scripts/quick_validate.ps1          # validate vs origin/main
#   powershell -File scripts/quick_validate.ps1 -All     # force full local matrix (rare)

param(
    [switch]$All,
    [string]$Base = "origin/main"
)

$ErrorActionPreference = "Stop"
$repo = Split-Path $PSScriptRoot -Parent
Set-Location $repo

# Node portable path used by this machine (kept here so the script just works)
$nodePortable = "C:\Users\DAFUNK~1\AppData\Local\Temp\opencode\node-portable\node-v22.14.0-win-x64"
if (Test-Path $nodePortable) { $env:Path = "$nodePortable;$env:Path" }

function Step($msg) { Write-Host "`n==> $msg" -ForegroundColor Cyan }
function Ok($msg)   { Write-Host "    OK  $msg" -ForegroundColor Green }
function Fail($msg) { Write-Host "    FAIL $msg" -ForegroundColor Red; exit 1 }

# -- collect changed files vs base -------------------------------------------
git fetch origin 2>$null | Out-Null
$baseRef = git rev-parse --verify "$Base" 2>$null
if (-not $baseRef) { $baseRef = git rev-parse --verify "HEAD~1" }
$changed = @(git diff --name-only $baseRef HEAD) + @(git diff --name-only) + @(git ls-files --others --exclude-standard)
$changed = $changed | Where-Object { $_ } | Sort-Object -Unique

if ($changed.Count -eq 0) { Write-Host "No changes detected vs $Base - nothing to validate."; exit 0 }

# -- classify -----------------------------------------------------------------
$isDocsOnly    = ($changed | Where-Object { $_ -notmatch '\.(md|txt|json)$' -and $_ -notmatch 'profiles/' }).Count -eq 0
$touchedPython = ($changed | Where-Object { $_ -match '^(treefrog-manager/(python|tests))/' }).Count -gt 0
$touchedPyFiles = @($changed | Where-Object { $_ -match '^treefrog-manager/tests/.*\.py$' })
$touchedFront   = ($changed | Where-Object { $_ -match '^treefrog-manager/src/.*\.(ts|tsx|css)$' }).Count -gt 0
$touchedRust    = ($changed | Where-Object { $_ -match '^treefrog-manager/src-tauri/src/.*\.rs$' -or $_ -match '^treefrog-manager/src-tauri/Cargo' }).Count -gt 0
$touchedVersion = ($changed | Where-Object { $_ -match '(Cargo\.toml|package\.json|tauri\.conf\.json)$' }).Count -gt 0

Write-Host "Changed files: $($changed.Count)"
$changed | ForEach-Object { Write-Host "  $_" }

if ($isDocsOnly -and -not $All) {
    Ok "Docs/context only - static review sufficient per DEC-2026-09-01-02. CI runs the full matrix on push."
    exit 0
}

$ran = 0

# -- Python: only affected test files -----------------------------------------
if ($touchedPython -or $All) {
    if (-not $touchedPyFiles -or $All) { $touchedPyFiles = @("treefrog-manager/tests") }
    Step "pytest - targeted"
    $targets = ($touchedPyFiles | ForEach-Object { $_ -replace '^treefrog-manager/', '' }) -join " "
    Push-Location treefrog-manager
    python -m pytest $targets -q 2>&1 | Out-Null
    $pyOk = $?
    Pop-Location
    if ($pyOk) { Ok "pytest PASS: $targets" } else { Fail "pytest failed: $targets" }
    $ran++
}

# -- Frontend: typecheck -------------------------------------------------------
if ($touchedFront -or $All) {
    Step "tsc --noEmit"
    Push-Location treefrog-manager
    npx tsc --noEmit 2>&1 | Out-Null
    $tscOk = $?
    Pop-Location
    if ($tscOk) { Ok "tsc PASS" } else { Fail "tsc --noEmit reported errors" }
    $ran++
}

# -- Rust: check + targeted tests when src changed ------------------------------
if ($touchedRust -or $All) {
    Step "cargo check"
    Push-Location treefrog-manager/src-tauri
    cargo check 2>&1 | Out-Null
    $cargoOk = $?
    if ($cargoOk) { Ok "cargo check PASS" } else { Pop-Location; Fail "cargo check failed" }
    # Targeted tests: only test modules matching changed src files
    $srcModules = $changed | Where-Object { $_ -match '^treefrog-manager/src-tauri/src/(\w+)\.rs$' } |
        ForEach-Object { $_ -replace '^treefrog-manager/src-tauri/src/', '' -replace '\.rs$', '' } | Sort-Object -Unique
    if ($srcModules) {
        $modList = $srcModules -join ', '
        Step "cargo test - targeted: $modList"
        $filter = ($srcModules | ForEach-Object { "$_::" }) -join "|"
        cargo test "$filter" 2>&1 | Select-String "test result" | Out-Null
        if ($?) { Ok "cargo targeted PASS" } else { Pop-Location; Fail "cargo targeted tests failed" }
    }
    Pop-Location
    $ran++
}

# -- Version consistency -------------------------------------------------------
if ($touchedVersion -or $All) {
    Step "Version consistency: Cargo = package.json = tauri.conf.json"
    $cargo = (Select-String -Path treefrog-manager/src-tauri/Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
    $pkg = (Get-Content treefrog-manager/package.json | ConvertFrom-Json).version
    $tauri = (Get-Content treefrog-manager/src-tauri/tauri.conf.json | ConvertFrom-Json).version
    if ($cargo -eq $pkg -and $pkg -eq $tauri) { Ok "versions consistent: $cargo" } else { Fail "version drift: cargo=$cargo pkg=$pkg tauri=$tauri" }
    $ran++
}

# -- Full matrix (only with -All) ------------------------------------------------
if ($All) {
    Step "FULL matrix -All"
    Push-Location treefrog-manager/src-tauri
    cargo fmt --check 2>&1 | Out-Null; if ($?) { Ok "cargo fmt" } else { Fail "cargo fmt --check" }
    cargo test 2>&1 | Select-String "test result: ok" | Out-Null; if ($?) { Ok "cargo test full" } else { Fail "cargo test full" }
    Pop-Location
    Push-Location treefrog-manager
    python -m pytest tests -q 2>&1 | Out-Null; if ($?) { Ok "pytest full" } else { Fail "pytest full" }
    npm run build 2>&1 | Out-Null; if ($?) { Ok "npm build" } else { Fail "npm build" }
    Pop-Location
    $ran++
}

if ($ran -eq 0) {
    Write-Host "`nNo code paths matched by minimal policy - CI runs the full matrix on push."
} else {
    $ranStr = "$ran"
    Write-Host "`nquick_validate: $ranStr check groups PASS. Push - CI executes the full gate." -ForegroundColor Green
}
exit 0
