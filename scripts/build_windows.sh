#!/usr/bin/env bash
# Reproducible Windows x64 build via WSL — documents limitation and delegates to Windows native
# Tauri 2 Windows builds require MSVC toolchain (Windows), not reliably cross-compiled from WSL/Ubuntu.
# This script documents the correct Windows-native command and attempts to detect prerequisites.

set -e
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANAGER_DIR="$REPO_ROOT/treefrog-manager"

echo "=== TreeFrog Content Manager — Windows x64 Build (WSL wrapper) ==="
echo "Repo: $REPO_ROOT"
echo "Manager: $MANAGER_DIR"
echo ""
echo "NOTE: Tauri Windows x64 builds require Windows-native MSVC toolchain."
echo "      Cross-compilation from WSL Ubuntu to Windows x64 is NOT reliable for the complete Tauri application"
echo "      (requires Windows SDK, WebView2, MSVC linker). This script documents the correct Windows-native build."
echo ""
echo "Windows-native build command (run in PowerShell from repo root):"
echo "  powershell -ExecutionPolicy Bypass -File scripts\\build_windows.ps1"
echo "  # or"
echo "  cd treefrog-manager && npm install && npm run build && npx tauri build"
echo ""
echo "Prerequisites (Windows):"
echo "  - Rust stable (https://rustup.rs) — cargo, rustc"
echo "  - Node.js 18+ (https://nodejs.org) — node, npm"
echo "  - Tauri CLI (installed via npm: npm install @tauri-apps/cli)"
echo "  - FFmpeg/ffprobe (https://ffmpeg.org, via winget: winget install Gyan.FFmpeg) — for video pipeline runtime"
echo "  - WebView2 (usually preinstalled on Windows 10/11) and MSVC Build Tools"
echo ""

# Check WSL prerequisites (for Linux build, not Windows)
echo "WSL prerequisites check (for Linux build, not Windows exe):"
for cmd in cargo rustc node npm ffmpeg ffprobe; do
  if command -v "$cmd" >/dev/null 2>&1; then
    echo "  $cmd: $($cmd --version 2>&1 | head -n 1)"
  else
    echo "  $cmd: NOT FOUND (expected on Windows, not required in WSL for Windows build)"
  fi
done

echo ""
echo "To build on Windows, open PowerShell and run:"
echo "  powershell -ExecutionPolicy Bypass -File scripts\\build_windows.ps1"
echo ""
echo "Artifacts (Windows):"
echo "  treefrog-manager/src-tauri/target/release/treefrog-manager.exe"
echo "  treefrog-manager/src-tauri/target/release/bundle/msi/*.msi"
echo "  treefrog-manager/src-tauri/target/release/bundle/nsis/*.exe"
echo ""
echo "If WSL has Rust/Node, you can attempt a Linux build for smoke test (not Windows exe):"
if command -v cargo >/dev/null 2>&1 && command -v npm >/dev/null 2>&1; then
  echo "  Attempting Linux build in WSL..."
  set +e
  cd "$MANAGER_DIR"
  npm install 2>&1 | tail -n 5
  npm run build 2>&1 | tail -n 10
  echo "WSL Linux build attempted (for smoke test only, not Windows exe)"
else
  echo "  Skipping WSL Linux build (missing cargo/node in WSL)"
fi

echo ""
echo "Documentation: see docs/BUILD_WINDOWS.md and treefrog-manager/README.md"
