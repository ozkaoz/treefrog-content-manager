# Auditoría TreeFrog Content Manager vs TreeFrogUI — 2026-09-01 (local)

## Hallazgos

### CRÍTICO 1 — Artwork `.res` escapa a la raíz de la SD (Rust + Python)
- `classify.rs:244` pone `destination: ".res"` para artwork dentro de `.res/`/`Imgs/`/`images/`.
- El planner concatena: `.res` + `.res/game.png` → `.res/.res/game.png` (raíz de la SD, duplicado).
- El validador Python/Rust **acepta** `.res/...` (no es ni absoluto ni traversal — pasa).
- Además: artwork NO debe desplegarse — es trabajo de Mini Scraper (contrato del producto: "Do NOT implement another scraper").
- **Fix**: artwork (`.res`, `Imgs/`, `images/`) → clasificar como `Kind::Image` pero con **skip explícito** (`action: skip`, reason: artwork gestionado por Mini Scraper). Nunca copiar.

### CRÍTICO 2 — BIOS patterns hardcoded en classify.rs
- `classify.rs:272-283` lista `scph`, `gba_bios.bin`, `o2rom.bin`, `disksys.rom`, `neogeo.zip`, `bios_cd`, `kick13/20.rom`, `pcfx.rom`, `x86boot.img` — duplica reglas de bios.json y viola el modelo declarativo.
- Riesgo: ROM llamado `scph-something.bin` → BIOS (false positive).
- `x86boot.img`: TreeFrogUI ya lo incluye → copiarlo duplicaría.
- **Fix**: eliminar el bloque hardcoded; el BIOS workflow ya es explícito (pestaña BIOS usa bios.json). En scan general, solo clasificar BIOS si el archivo está DENTRO de una carpeta `cubegm/bios` del source o coincide con bios.json patterns cargados dinámicamente del perfil.

### ALTO 3 — Sistema Vectrex (vec) falta en systems.json
- Upstream tiene `vec` → `vecx_libretro.so` (`.vec` files). Nuestro perfil NO lo tiene.

### ALTO 4 — Extensiones faltantes
- `wsv`: falta `.wsv`
- `retro8`: falta `.p8.png`
- `lowres-nx`: falta `.nx` (solo tiene `.lowresnx`)
- `gw`: falta `.gw` (solo `.mgw`)
- `flashback`: faltan `.abi`, `.epr`

### MEDIO 5 — `fake08` alias en pico8
- Upstream dice "legacy fake08 folder still works" — nuestro alias es correcto, OK. No action.

### Observaciones OK (sin acción)
- Cores: 0 mismatches contra tabla oficial.
- BIOS set (13) coincide con doc oficial + pc88/pcfx.
- media.json music/videos/images/ebooks coincide con el upstream (incl. `roms/Ebook` destino y `.positions` intocado).
- Music playlists, videos hw-decoder, LGPT paths — todos verificados correctos v1.0.1.
