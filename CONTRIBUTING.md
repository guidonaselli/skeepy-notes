# Contribuir a Skeepy

## Requisitos

- [Rust stable](https://rustup.rs/) (1.75+)
- [Node.js](https://nodejs.org/) 22+
- [Tauri CLI v2](https://v2.tauri.app/start/prerequisites/): `cargo install tauri-cli --version "^2"`
- Windows 10+ con WebView2 instalado

## Levantar el entorno de desarrollo

```bash
# Instalar dependencias del frontend
npm install

# Levantar en modo dev (hot-reload frontend + backend compilado en debug)
cargo tauri dev
```

La ventana de Skeepy se abre automáticamente. Los cambios en `src/` se reflejan al instante. Los cambios en `src-tauri/` requieren reinicio del proceso Rust (automático).

## Correr los tests

```bash
# Tests de todos los crates del workspace
cargo test --workspace

# TypeScript check
npx tsc --noEmit
```

## Arquitectura

La arquitectura completa está documentada en `.gsd/milestones/M001/M001-CONTEXT.md`.

En resumen:

```
src-tauri/crates/core/       → Domain layer (traits, entidades, sin I/O)
src-tauri/crates/storage/    → SQLite + FTS5 (implementa repos del domain)
src-tauri/crates/providers/  → Implementaciones de NoteProvider (local, keep, ...)
src-tauri/src/               → Tauri shell (IPC commands, state, tray)
src/                         → Solid.js frontend
```

## Agregar un nuevo provider

1. Crear un nuevo crate en `src-tauri/crates/providers/<nombre>/`
2. Implementar el trait `NoteProvider` de `skeepy-core`
3. Registrar el provider en `src-tauri/src/lib.rs` (en el `setup`)
4. Agregar IPC commands de autenticación si el provider los necesita

## Variables de entorno

Para compilar con credenciales de Google Keep embebidas:

```bash
GOOGLE_CLIENT_ID=tu-client-id GOOGLE_CLIENT_SECRET=tu-secret cargo tauri build
```

Sin estas variables, el binario compila igual pero los usuarios necesitan ingresar sus propias credenciales en Settings.

## Convenciones

- Commits en [Conventional Commits](https://www.conventionalcommits.org/)
- Un PR = un concern
- Cada PR debe pasar `cargo test --workspace` y `npx tsc --noEmit`
- Decisiones de arquitectura van en `.gsd/DECISIONS.md` (append-only)
