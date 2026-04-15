# S14: Markdown Folder Provider

**Goal:** El usuario puede agregar una carpeta como "provider Markdown" desde la UI.

**After this:** Cada archivo `.md` en la carpeta configurada se convierte en una nota. Título = nombre del archivo (sin extensión). Cuerpo = contenido. Frontmatter YAML opcional para title/color/tags. Watch de cambios con `notify`.

---

## Tasks

- [x] T01 — Crear `skeepy-provider-markdown` crate
- [x] T02 — Implementar lectura de archivos .md (sin frontmatter)
- [x] T03 — Parsear frontmatter YAML (title, color, tags)
- [x] T04 — IPC: `markdown_set_folder` / `markdown_get_folder`
- [x] T05 — Registrar provider en AppState dinámicamente al configurar la carpeta

## Must-Haves

- Si la carpeta no existe o no está configurada → provider devuelve `[]` sin error
- No recursivo por defecto (solo primer nivel)
- Ignorar archivos que empiecen con `.` o `_`
- `source_id` = hash SHA256 del path relativo (estable aunque cambie el contenido)
- Frontmatter delimitado por `---` al inicio del archivo

## No incluye en V1

- File watcher en tiempo real (se deja para V3 — sync periódico es suficiente)
- Soporte recursivo de carpetas
