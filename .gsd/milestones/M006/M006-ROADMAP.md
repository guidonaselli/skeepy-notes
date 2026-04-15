# M006: V3.5 — "Plataforma abierta"

**Vision:** Skeepy se convierte en una plataforma extensible. Notion y Obsidian se suman como
providers de primera clase. Terceros pueden publicar sus propios providers como plugins instalables.
El usuario puede exportar sus notas y gestionar labels desde la UI.

**Success Criteria:**
- Notion se puede conectar como provider de lectura y escritura
- Una vault de Obsidian se puede agregar como provider local
- Un desarrollador externo puede crear y publicar un provider sin modificar el código de Skeepy
- El usuario puede exportar todas sus notas a JSON, Markdown o PDF
- El usuario puede crear, renombrar y borrar labels/tags desde la UI

---

## Slices

- [ ] **S26: Notion Provider** `risk:high` `depends:[S16, S19]`
  > After this: El usuario conecta Notion via OAuth2 y sus páginas de Notion aparecen
  > en Skeepy. El provider soporta lectura y escritura. Usa la Notion API v2 con
  > autenticación OAuth2 (no Internal Integration Tokens).

- [ ] **S27: Obsidian Provider** `risk:low` `depends:[S14]`
  > After this: El usuario selecciona su vault de Obsidian como carpeta y las notas
  > aparecen en Skeepy. Es esencialmente una extensión del Markdown Folder Provider (S14)
  > con soporte específico para el frontmatter de Obsidian (tags, aliases, created date)
  > y el formato de backlinks `[[nombre de nota]]`.

- [ ] **S28: Plugin System (WASM)** `risk:very-high` `depends:[S16, S26, S27]`
  > After this: Existe una API pública de plugins que permite a terceros implementar
  > el trait `NoteProvider` como una WASM module. Los plugins se pueden instalar desde
  > una URL o un registry. Skeepy carga y ejecuta los WASM modules en un sandbox seguro.
  > Este es el slice más complejo del roadmap — requiere diseño cuidadoso del ABI.

- [ ] **S29: Export** `risk:low` `depends:[S15]`
  > After this: El usuario puede exportar todas sus notas (o las de un provider específico)
  > a JSON, Markdown (una nota por archivo, con frontmatter), o PDF.
  > Export a JSON: compatible con el formato de import del Local Provider.
  > Export a Markdown: carpeta de archivos `.md` nombrados por título de nota.
  > Export a PDF: cada nota en una página, usando una renderer HTML → PDF (wkhtmltopdf o similar).

- [ ] **S30: Labels/Tags Management UI** `risk:low` `depends:[S15]`
  > After this: Existe una vista "Labels" en la UI donde el usuario puede:
  > - Ver todas las labels existentes (de todos los providers)
  > - Crear labels locales nuevas
  > - Renombrar labels locales
  > - Borrar labels locales (no afecta a labels del provider remoto)
  > - Filtrar notas por label desde la NoteGrid

- [ ] **S31: Conflict Resolution UI** `risk:medium` `depends:[S20, S24]`
  > After this: Cuando se detecta un conflicto (nota editada en Skeepy y en el provider
  > remoto simultáneamente), se muestra un diff visual con las dos versiones y el usuario
  > elige cuál conservar o puede hacer un merge manual.

---

## Research Needed

### Notion API v2

**Endpoint base:** `https://api.notion.com/v1`

**Operaciones:**
- `POST /oauth/token` — intercambiar code por tokens
- `GET /search` — buscar páginas y databases
- `GET /databases/{id}/query` — listar páginas de una database
- `GET /pages/{id}` — metadata de una página
- `GET /blocks/{id}/children` — contenido de una página (bloques)
- `PATCH /blocks/{id}` — editar un bloque
- `POST /pages` — crear página

**Challenge importante:** El contenido de Notion es un árbol de bloques, no texto plano.
La conversión a `NoteContent::Text` implica pérdida de información. Para V3.5, hacer la
conversión lo más fiel posible (párrafos + listas + headers → texto).

**OAuth2 para Notion:**
- Flujo estándar Authorization Code (sin PKCE — Notion no lo soporta)
- Redirect URI: `http://localhost:PORT` (igual que Keep y OneNote)
- Scopes: `read_content`, `update_content`, `insert_content`

### Plugin System con WASM

El diseño del ABI es el challenge más grande de todo el roadmap.

**Opción 1: wasmtime + Component Model**
- Usar `wasmtime` para ejecutar WASM modules con el Component Model de WebAssembly
- Definir el interface en WIT (WebAssembly Interface Types)
- Los plugins implementan el WIT interface → se compilan a WASM → Skeepy los carga
- Pro: sandbox real, tipo-safe, lenguaje-agnóstico
- Con: wasmtime agrega ~10MB al binario, complejidad alta del ABI design

**Opción 2: Dynamic libraries (.dll)**
- Los plugins son .dll que exportan funciones C con un ABI definido
- Skeepy las carga con `libloading` crate
- Pro: más simple, más rápido
- Con: sin sandbox, solo Windows, puede crashear el proceso host

**Recomendación:** Opción 1 (wasmtime). Es más compleja pero es el único approach
que permite plugins de terceros seguros en producción.

**WIT interface tentativa:**
```wit
interface note-provider {
    record remote-note { ... }
    fetch-notes: func(since: option<u64>) -> result<list<remote-note>, string>
    create-note: func(req: create-request) -> result<remote-note, string>
}
```

### Export a PDF

Opciones en Rust:
1. `printpdf` crate — generación de PDF programática. Limitado en layout complejo.
2. Render HTML → imprimir a PDF via `tauri::WebviewWindow::print()` (usa WebView2 print dialog)
3. `headless_chrome` / `chromium` — demasiado pesado

**Recomendación para V3.5:** Opción 2 (WebView2 print to PDF). Tauri 2.x permite
`window.print_to_pdf()` que usa el motor de impresión de WebView2 — produce PDFs de
buena calidad sin dependencias adicionales.

---

## Orden recomendado

S29 (Export) → S30 (Labels UI) → S27 (Obsidian) → S26 (Notion) → S31 (Conflicts) → S28 (Plugins)

S28 es el último porque es el más complejo y depende de tener múltiples providers
implementados para validar que el ABI es suficientemente genérico.
