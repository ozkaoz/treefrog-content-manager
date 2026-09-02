# F8 - Harness de regresion funcional en host
> **⚠️ DOC HEREDADO del port LGPT (lgpt-r36sx-port) — NO aplica al Content Manager.**
> Este documento vino con la historia del repositorio. El Content Manager es una app
> Tauri 2 (npm + cargo). Para compilar el port LGPT, ver `D:\GitHub\lgpt-r36sx-port`
> y `docs/TOOLCHAINS.md` de ese repo (rutas: `/mnt/d/Toolchains/R36SX`).

Tramo del refactor `refactor/bacon-1.2.1-preserve` (golden Bacon 1.2.1).
Sin cambio de binario: el core se mantiene en el sha `ea7a80e4` y el
daemon en `4be71632` (goldens de F7).  Lo que entrega F8 es el harness
host que inyecta inputs EPBM_* y comprueba el comportamiento catalogado
por vista contra el resolver dorado.

## Contexto: por que no hay build X64 completo

El plan original del tramo pedia el core compilable en x86_64
(`make -f Makefile.X64`) con backend null/SDL para poder correr el
pipeline completo (retro_run -> vistas -> ActionId) en host.  Ese camino
esta bloqueado en este entorno:

- WSL sin `pkg-config`, `libsdl2-dev`, `libasound2-dev` ni
  `libjack-jackd2-dev`; `apt-get install` no completa (timeouts de 900s
  y 1800s, red no disponible).
- Las rutas con espacios rompen el Makefile (`-include $(PWD)/rules_base`
  -> `*** /mnt/d/R36S/PORT: Is a directory`); el symlink `/tmp/lgpt_src`
  resuelve la ruta pero sin toolchain host el build no llega a objeto.

Alternativa adoptada: el pipeline F1/F2 (ActionMap + ChordResolver) ya
compila standalone en host y materializa la fuente de verdad del input
dorado.  F8 construye sobre el el harness de regresion funcional, que
cubre el mismo objetivo (inyectar input y verificar comportamiento) a
nivel de la capa que concentra toda la logica de input de las vistas.

## Que se hizo

### 1. ScenarioCatalog.h (base de datos de escenarios por vista)

`source/sources/Application/UI/Input/ScenarioCatalog.h` es una capa pura
(solo ActionId.h y ChordResolver.h; prohibido GUI/audio/daemons/POSIX)
que declara la BD declarativa de escenarios:

```
struct Scenario {
    const char *view;     // vista donde ocurre el escenario
    ContextId ctx;        // contexto de resolucion
    PadMask mask;         // mascara EPBM_* inyectada
    ActionId expected;    // primera accion (la del resolver)
    ActionId queued;      // segunda accion de la cola multi-fire
    const char *doc;      // rama del golden que transcribe
};
```

56 escenarios que cubren los 6 contextos (CTX_GLOBAL, CTX_MIXER,
CTX_MIXER_FX, CTX_CHOPPER, CTX_CHOPPER_TRIM, CTX_CHOPPER_PITCH).
A diferencia de F1 (que transcribe la tabla plana de bindings), F8
cataloga el comportamiento DE ORDEN SUPERIOR:

- Secuencias multi-fire documentadas del golden (MixerView.cpp:672-696:
  A+UP/L1+UP -> coarse vol con cola de flechas, START -> PLAY_STOP con
  cola de cursor, etc.).
- Requisitos estables del port (pitch B/A puro = preview/apply,
  trim R1+A = crop, chopper B/A = preview/add, ...).
- Negaciones documentadas (SELECT y R2+A puros en pitch sin accion;
  R2 puro en paginas FX sin accion).
- Cada fila referencia la rama dorada (archivo:lineas) que transcribe.

Cada accion esperada existe en ActionId.h (sin nombres inventados).

### 2. scenario_runner_host_test.cpp (runner)

`tests/host/scenario_runner_host_test.cpp` inyecta cada mascara en
`ChordResolver_Resolve` y verifica 5 invariantes:

1. La accion golden: `resolve(mask, ctx) == expected`.
2. Determinismo: 3 resoluciones identicas.
3. Unicidad: ningun (ctx, mascara) duplicado en el catalogo.
4. Coherencia catalogo <-> ActionMap: la accion esperada existe en el
   ActionMap del contexto declarado y AL MENOS UN binding (los hay
   multi-cord: A+UP y L1+UP -> VOLUME_COARSE_UP; START y START+UP ->
   PLAY_STOP) cubre la mascara; las negaciones no colisionan con un
   binding real.
5. Cobertura: los 6 contextos tienen escenarios.

Compila con ASAN/UBSAN vía `tests/run_host_action_scenarios.sh`
(anadido a `scripts/audit.sh`).  Resultado:
`ACTION_SCENARIOS_HOST_ALL_OK (342 checks)`.

### 3. test_f8_baseline.py (baseline estatico)

`tests/test_f8_baseline.py` verifica: estructura del catalogo y
cobertura de contextos; pureza de la capa (0 dependencias prohibidas);
que el catalogo NO duplica el ActionMap (transcribe menos que la tabla
de bindings y de orden superior); que el runner esta registrado en
audit.sh con ASAN/UBSAN; que todo ActionId del catalogo existe en
ActionId.h; y que la limitacion del build X64 queda documentada.
Resultado: `F8_BASELINE_OK (64 checks)`.

## Evidencia

- Audit `AUDIT_CLEAN_MAIN_U2523_OK` (F8_BASELINE_OK + ACTION_SCENARIOS_HOST_ALL_OK).
- Core MIPS `ea7a80e4` + daemon `4be71632` reconstruidos y desplegados en
  SD == build (sin cambio de binario; F8 solo anade codigo host).
- Gate diag `NO_DIAGNOSTICS_OUTSIDE_DEVICE`; set de warnings identico al
  build post-F7 (los 4 preexistentes en device/*.c: 534 y 70 del daemon
  u2523, 2809 y 76 del sp404).
- Backup `LGPT_BEFORE_U2523_20260813_235903`.

## Deuda documentada

- Build X64 del core en host bloqueado por dependencias (pkg-config,
  SDL2, ALSA, JACK) no instalables en este WSL; el harness F8 sobre
  ActionMap/ChordResolver es la unica via de regresion funcional de
  input en host hasta que el entorno lo permita.
