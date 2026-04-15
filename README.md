# Skeepy Notes

Agregador de notas local para Windows. Muestra tus notas de Google Keep (y más providers próximamente) como sticky notes en tu escritorio, vive en el system tray, consume < 50MB de RAM en idle y arranca en menos de 1 segundo.

**No es un cliente de Google Keep.** Keep es un provider opcional — la app funciona perfectamente sin él.

---

## Características

- Notas como cards arrastrables con posición y tamaño persistente
- Búsqueda full-text ultrarrápida (SQLite FTS5, < 50ms en 1000 notas)
- Google Keep como provider de lectura (OAuth2, sin servidor propio)
- Notas locales via archivo JSON
- Vive en el system tray, arranca con Windows
- Sin telemetría, sin cuenta requerida, datos 100% locales

## Requisitos

- Windows 10 o superior (WebView2 viene preinstalado desde Windows 10)
- No requiere permisos de administrador

## Instalación

1. Descargá el instalador desde [Releases](../../releases/latest)
2. Ejecutá `Skeepy_x.x.x_x64-setup.exe` (instalación en modo usuario, sin admin)
3. La app aparece en el system tray — buscá el ícono en la barra de tareas

## Notas locales

Creá un archivo `notes.json` en:

```
%APPDATA%\com.skeepy.notes\notes.json
```

Formato de ejemplo:

```json
[
  {
    "id": "1",
    "title": "Mi primera nota",
    "content": "Hola mundo",
    "color": "yellow",
    "pinned": false
  }
]
```

## Conectar Google Keep

1. Abrí Skeepy → click derecho en el tray → **Mostrar Skeepy**
2. Andá a **Configuración** (ícono de engranaje)
3. En la sección **Google Keep**, hacé click en **Conectar Google Keep**
4. Tu browser se abre con la pantalla de autorización de Google
5. Aceptá los permisos → la app se conecta automáticamente

> **Nota:** La app usa el scope `keep.readonly` — solo lectura, nunca escribe ni elimina tus notas de Keep.

> **Usuarios avanzados:** Si querés usar tu propia app de Google Cloud Console, expandí "Credenciales personalizadas" en Settings y pegá tu Client ID y Client Secret.

## Privacidad

- Todos los datos se almacenan localmente en `%APPDATA%\com.skeepy.notes\`
- Los tokens de Google se guardan en Windows Credential Manager (cifrado por el OS)
- Skeepy no tiene servidor propio — el flujo OAuth es directo entre tu PC y Google
- Sin telemetría, sin analytics, sin datos enviados a terceros

## Compilar desde el código fuente

Ver [CONTRIBUTING.md](CONTRIBUTING.md).

## Licencia

MIT — ver [LICENSE](LICENSE)
