# Instalación por copia directa: U2.52.4 copy-root + ALSA/UAC2 validado
> **⚠️ DOC HEREDADO del port LGPT (lgpt-r36sx-port) — NO aplica al Content Manager.**
> Este documento vino con la historia del repositorio. El Content Manager es una app
> Tauri 2 (npm + cargo). Para compilar el port LGPT, ver `D:\GitHub\lgpt-r36sx-port`
> y `docs/TOOLCHAINS.md` de ese repo (rutas: `/mnt/d/Toolchains/R36SX`).

## Causa confirmada

El selector de muestras de LGPT comprueba que el valor `SAMPLELIB` sea un directorio. La configuración Stock apunta a `/mnt/sdcard/lgpt/samples`. Los ZIP y Git no conservan directorios vacíos; además, el launcher anterior no provisionaba todas las rutas relacionadas. El resultado visible es `Can't access the samplelib`.

Este paquete corrige ambos puntos:

1. Incluye archivos `.keep` para conservar la estructura en Git/ZIP.
2. El launcher crea y verifica `samples`, `samples/records`, `samplelib`, `instruments`, `projects`, `tmp/record` y demás rutas antes de iniciar el core.

## Copia manual desde Windows, sin ejecutar instaladores

Con Stock OS + TreeFrogUI + el port actualmente funcional ya presentes:

1. Extraiga el ZIP en `D:\R36S\PORT LPTRACKER\LGPT_R36SX_U2524_COPYROOT_UAC2`.
2. Copie las carpetas `cubegm`, `lgpt`, `roms` y `LGPT_OTG_LOGS` a `F:\`.
3. Acepte combinar carpetas y reemplazar `cubegm\lgpt`, `lgpt\config.xml`, `lgpt\config.stock.xml` y `roms\lgpt\start.lgpt`.
4. Expulse la SD de forma segura desde Windows antes de retirarla.

No copie `SOURCE_AND_TOOLS` a la SD; no es necesario para ejecutar el port.

## Instalación y verificación desde WSL

```bash
cd "/mnt/d/R36S/PORT LPTRACKER/LGPT_R36SX_U2524_COPYROOT_UAC2"
bash SOURCE_AND_TOOLS/tests/run_all.sh
bash SOURCE_AND_TOOLS/scripts/install_to_sd_f.sh
```

El segundo comando monta `F:` en `/mnt/f`, crea un respaldo, copia sólo el payload, verifica rutas/configuración, ejecuta `sync` y desmonta la SD.

## Prueba exacta en R36S

1. Inicie LGPT desde TreeFrogUI.
2. Entre a `Instrument` y seleccione un instrumento `Sample`.
3. Coloque el cursor en `sample`.
4. Pulse `A` para entrar al modo de selección y `A` nuevamente para abrir el navegador.
5. Resultado esperado: abre el navegador de muestras; no aparece `Can't access the samplelib`.
6. Apague o salga de la aplicación de forma limpia antes de retirar la SD.

## Extracción de logs después de la prueba

```bash
cd "/mnt/d/R36S/PORT LPTRACKER/LGPT_R36SX_U2524_COPYROOT_UAC2"
bash SOURCE_AND_TOOLS/scripts/collect_logs_from_sd_f.sh
```

El comando monta la SD, copia `LGPT_OTG_LOGS` y `lgpt/otg/logs`, guarda un inventario de estado, crea un `.tar.gz`, ejecuta `sync` y desmonta la SD.

## Límite deliberado de este hotfix

Este paquete no reemplaza ni fabrica el binario ARM del core. Conserva el core funcional que ya está en `F:\cubegm\cores\lgpt_r36sx_port_libretro.so`. Una futura publicación limpia para una SD que sólo tenga Stock + TreeFrogUI debe incorporar ese binario compilado, el daemon OTG y el módulo del kernel a la estructura de release después de construirlos desde el repositorio completo.

## Generación de release desde el clone completo

Después de aplicar `SOURCE_AND_TOOLS/upstream_overlay` al clone completo, el generador bloquea un release autónomo si faltan el core, el daemon OTG o el módulo del kernel:

```bash
cd "/mnt/d/R36S/PORT LPTRACKER/lgpt-r36sx-port"
bash scripts/build_from_full_clone.sh
```

Para generar deliberadamente un hotfix que conserva los binarios ya instalados en la SD:

```bash
HOTFIX_OVERLAY_ONLY=1 bash scripts/build_from_full_clone.sh
```

El ZIP generado incluye el payload en su raíz y una copia del repositorio fuente completo bajo `SOURCE_AND_TOOLS/full_repository`.
