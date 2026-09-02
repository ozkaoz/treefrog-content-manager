# English build guide
> **⚠️ DOC HEREDADO del port LGPT (lgpt-r36sx-port) — NO aplica al Content Manager.**
> Este documento vino con la historia del repositorio. El Content Manager es una app
> Tauri 2 (npm + cargo). Para compilar el port LGPT, ver `D:\GitHub\lgpt-r36sx-port`
> y `docs/TOOLCHAINS.md` de ese repo (rutas: `/mnt/d/Toolchains/R36SX`).

## Requirements

- WSL/Ubuntu.
- `rsync`, `make`, `gcc`, `g++`, `python3`.
- R36SX MIPS toolchain:

```text
$HOME/sf3000-work/sf3000toolchain/mipsel-buildroot-linux-gnu_sdk-buildroot
```

## Audit, build and install

```bash
bash scripts/audit.sh
PROJECT_ROOT="/mnt/d/R36S/PORT LPTRACKER" bash scripts/build.sh
SD_MOUNT=/mnt/f PROJECT_ROOT="/mnt/d/R36S/PORT LPTRACKER" bash scripts/install.sh
SD_MOUNT=/mnt/f PROJECT_ROOT="/mnt/d/R36S/PORT LPTRACKER" bash scripts/verify.sh
```

Full workflow:

```bash
SD_MOUNT=/mnt/f PROJECT_ROOT="/mnt/d/R36S/PORT LPTRACKER" bash scripts/build_install.sh
```
