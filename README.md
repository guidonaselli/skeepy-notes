# Skeepy Notes

Agregador de notas local y de escritorio. Unifica tus notas de Google Keep, Microsoft OneNote, Notion, Obsidian y carpetas Markdown en un solo lugar, como sticky notes en tu escritorio. Vive en el system tray, consume < 50 MB de RAM en idle y arranca en menos de 1 segundo.

**No es un cliente de Google Keep.** Keep es un provider opcional — la app funciona perfectamente sin ningún provider externo.

---

## Características

- **Providers soportados:** notas locales, Google Keep, Microsoft OneNote, Notion, Obsidian Vault, Carpeta Markdown, Windows Sticky Notes (solo Windows)
- **Sticky notes en el escritorio** — ventanas nativas sin decoración, arrastrables, redimensionables, siempre visibles (pinned), con color propio
- **Búsqueda full-text** ultrarrápida (SQLite FTS5, < 50ms en 1000 notas)
- **Búsqueda semántica** (TF-IDF, toggle desde la barra de búsqueda)
- **Escritura y edición** de notas en providers que lo soportan (local, Keep, OneNote, Notion)
- **Resolución de conflictos** — UI diff local vs. remoto cuando hay edición simultánea
- **Labels / Tags** — filtro por etiqueta, renombrar, eliminar
- **Export** a JSON o Markdown
- **Auto-update** — notificación en tray y banner en manager cuando hay nueva versión
- **Smart Sync Scheduler** — sincronización adaptativa según tu historial de uso
- System tray, autostart con el sistema, sin cuenta requerida, sin telemetría

## Plataformas

| OS | Soporte |
|----|---------|
| Windows 10+ | Completo (WebView2 preinstalado) |
| macOS 10.15+ | Completo (sin Sticky Notes provider) |
| Linux | Completo (sin Sticky Notes provider) |

## Instalación

1. Descargá el instalador desde [Releases](../../releases/latest)
2. **Windows:** ejecutá `Skeepy_x.x.x_x64-setup.exe` (instalación en modo usuario, sin admin)
3. **macOS:** abrí el `.dmg` y arrastrá la app a Aplicaciones
4. La app aparece en el system tray — buscá el ícono en la barra de tareas

Al iniciar por primera vez, el manager se abre con una pantalla de bienvenida donde podés conectar tus providers. Si tenías sticky notes abiertas en la sesión anterior, se restauran automáticamente.

## Providers

### Notas locales

Creá notas directamente en la app (Ctrl+N o el botón ＋). Se guardan en:

```
%APPDATA%\com.skeepy.notes\   (Windows)
~/Library/Application Support/com.skeepy.notes/   (macOS)
~/.local/share/com.skeepy.notes/   (Linux)
```

### Google Keep

1. Abrí Settings (⚙) → sección **Google Keep** → **Conectar**
2. Tu browser se abre con la autorización de Google
3. Aceptá los permisos → la app se conecta automáticamente

> Usa el scope `keep.readonly` — solo lectura desde Keep. Podés crear y editar notas localmente.

> **Usuarios avanzados:** expandí "Credenciales personalizadas" en Settings para usar tu propio proyecto de Google Cloud Console.

### Microsoft OneNote

1. Settings → **Microsoft OneNote** → **Conectar**
2. Iniciá sesión con tu cuenta Microsoft

> **Usuarios avanzados:** podés registrar tu propia app en Azure Portal y usar tu Application ID.

### Notion

1. Registrá una integración pública en [notion.so/profile/integrations](https://notion.so/profile/integrations)
2. Settings → **Notion** → expandí "Credenciales de integración" → pegá tu Client ID, Client Secret y Parent Page ID
3. Hacé click en **Conectar**

### Carpeta Markdown

Settings → **Carpeta Markdown** → pegá la ruta a tu carpeta. Todos los `.md` se importan como notas.

### Obsidian Vault

Settings → **Obsidian Vault** → pegá la ruta a tu vault. Los backlinks `[[…]]` se convierten a texto y los `#tags` inline se extraen como labels.

### Windows Sticky Notes

Se detecta automáticamente en Windows — no requiere configuración. Importa las notas del sistema desde la base de datos de Sticky Notes.

## Privacidad

- Todos los datos se almacenan localmente
- Los tokens OAuth se guardan en el keychain del sistema (Windows Credential Manager / macOS Keychain / libsecret en Linux) — nunca en texto plano
- Skeepy no tiene servidor propio — el flujo OAuth es directo entre tu PC y el provider
- Sin telemetría, sin analytics, sin datos enviados a terceros

## Compilar desde el código fuente

Ver [CONTRIBUTING.md](CONTRIBUTING.md).

## Licencia

MIT — ver [LICENSE](LICENSE)
