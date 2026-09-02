# Solución de problemas
> **⚠️ DOC HEREDADO del port LGPT (lgpt-r36sx-port) — NO aplica al Content Manager.**
> Este documento vino con la historia del repositorio. El Content Manager es una app
> Tauri 2 (npm + cargo). Para compilar el port LGPT, ver `D:\GitHub\lgpt-r36sx-port`
> y `docs/TOOLCHAINS.md` de ese repo (rutas: `/mnt/d/Toolchains/R36SX`).

## WSL no puede acceder a `/mnt/f`

Cierre WSL, ejecute `wsl --shutdown` en PowerShell y vuelva a abrir. Compruebe `findmnt -T /mnt/f`.

## Windows no reconoce OTG

Apague completamente la consola, conecte un cable USB-C de datos y vuelva a encender. Verifique el dispositivo `R36SX USB AUDIO 48K`.

## Restaurar la versión previa

```bash
SD_MOUNT=/mnt/f PROJECT_ROOT="/mnt/d/R36S/PORT LPTRACKER" bash scripts/restore.sh
```

## Recolectar logs

```bash
SD_MOUNT=/mnt/f PROJECT_ROOT="/mnt/d/R36S/PORT LPTRACKER" bash scripts/collect_logs.sh
```
