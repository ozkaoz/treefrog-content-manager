# Troubleshooting
> **⚠️ DOC HEREDADO del port LGPT (lgpt-r36sx-port) — NO aplica al Content Manager.**
> Este documento vino con la historia del repositorio. El Content Manager es una app
> Tauri 2 (npm + cargo). Para compilar el port LGPT, ver `D:\GitHub\lgpt-r36sx-port`
> y `docs/TOOLCHAINS.md` de ese repo (rutas: `/mnt/d/Toolchains/R36SX`).

## WSL cannot access `/mnt/f`

Close WSL, run `wsl --shutdown` in PowerShell, and reopen it. Check `findmnt -T /mnt/f`.

## Windows does not detect OTG

Fully power off, connect a USB-C data cable, and boot again. Select `R36SX USB AUDIO 48K`.

## Restore the previous version

```bash
SD_MOUNT=/mnt/f PROJECT_ROOT="/mnt/d/R36S/PORT LPTRACKER" bash scripts/restore.sh
```

## Collect logs

```bash
SD_MOUNT=/mnt/f PROJECT_ROOT="/mnt/d/R36S/PORT LPTRACKER" bash scripts/collect_logs.sh
```
