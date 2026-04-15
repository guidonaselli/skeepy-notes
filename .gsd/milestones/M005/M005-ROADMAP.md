# M005: V3.0 — "El ecosistema Microsoft"

**Vision:** Skeepy integra el ecosistema Microsoft completo: OneNote con lectura y escritura,
MSIX para el Microsoft Store, y firma de código OV. La app es una ciudadana de primera clase
del ecosistema Windows.

**Success Criteria:**
- OneNote se puede conectar y las notas aparecen en Skeepy (lectura)
- El usuario puede crear y editar notas en OneNote desde Skeepy (escritura)
- La app está disponible en el Microsoft Store y se puede instalar desde ahí
- Proceso de review del Store completado

---

## Slices

- [ ] **S23: OneNote Provider — Lectura** `risk:high` `depends:[S16]`
  > After this: El usuario conecta su cuenta Microsoft desde Settings, y sus notas
  > de OneNote aparecen en Skeepy (lectura). Usa Microsoft Graph API con OAuth2 + MSAL.
  > Importa notebooks, sections, pages — cada page es una nota en Skeepy.

- [ ] **S24: Write Support — OneNote** `risk:high` `depends:[S23, S19]`
  > After this: El usuario puede crear y editar páginas de OneNote desde Skeepy.
  > Usa Microsoft Graph API `PATCH /me/onenote/pages/{id}/content`.
  > El editor de S19 funciona para OneNote con las limitaciones del formato OneNote.

- [ ] **S25: MSIX + Microsoft Store** `risk:medium` `depends:[S22]`
  > After this: La app está disponible en el Microsoft Store. El packaging MSIX se genera
  > en CI junto con el NSIS. El proceso de review de Microsoft está completado.
  > Los usuarios de Store reciben updates automáticamente via Store.

---

## Research Needed

### OneNote via Microsoft Graph API

**Endpoint base:** `https://graph.microsoft.com/v1.0/me/onenote`

**Operaciones disponibles:**
- `GET /notebooks` — listar notebooks
- `GET /notebooks/{id}/sectionGroups` — grupos de secciones
- `GET /notebooks/{id}/sections` — secciones
- `GET /sections/{id}/pages` — páginas (lista, sin contenido)
- `GET /pages/{id}/content` — contenido HTML de una página
- `POST /sections/{id}/pages` — crear página (HTML content)
- `PATCH /pages/{id}/content` — actualizar página (patch commands JSON)
- `DELETE /pages/{id}` — eliminar página

**OAuth2 — MSAL para desktop apps:**
- Authority: `https://login.microsoftonline.com/common`
- Scopes: `Notes.Read`, `Notes.ReadWrite` (no `.All` — solo notebooks del usuario)
- Flow: Authorization Code + PKCE (igual que Google, misma arquitectura)
- Client ID: registrar app en Azure Portal (App Registration) — gratuito
- El callback local funciona igual que con Google: `http://localhost:PORT`
- Token storage: keyring crate (DPAPI) — misma implementación que Keep

**Diferencias vs Google Keep:**
- OneNote soporta edición (`PATCH`) — no hay la limitación de Keep
- El contenido de las páginas es HTML (no texto plano) — necesita conversión a/desde NoteContent
- Paginación diferente: usa `@odata.nextLink` en lugar de `pageToken`
- Rate limits: 10.000 requests/día por app, 120 requests/min por usuario

**Challenge:** El contenido de OneNote es HTML enriquecido. La conversión a `NoteContent::Text`
pierde formato. Para V3.0, convertir HTML → texto plano. Para V3.5+ se podría agregar
`NoteContent::RichText(html: String)` al domain model.

### MSIX Packaging con Tauri

En `tauri.conf.json`:
```json
"bundle": {
  "targets": ["nsis", "msi"]
}
```
El target `msi` genera un MSIX/WiX que Microsoft Store acepta.
Requiere cuenta de Microsoft Partner Center (~$20 one-time para individuos, gratis para empresas).
Certificación: 1-3 días hábiles típicamente.

**Cambio en el manifest:** El MSIX sandbox puede tener restricciones en el acceso al filesystem.
Verificar que `%LocalAppData%\Packages\<id>` sigue siendo accesible (debe serlo — es el AppData del sandbox).

---

## Decisión Pendiente: Microsoft Identity Model

OneNote usa Microsoft Identity Platform (MSAL), no OAuth2 estándar.
Opciones para la implementación:
1. `microsoft-kiota-authentication-oauth` crate (oficial, pesado)
2. Implementar el flow PKCE directamente con `reqwest` (igual que Keep) — recomendado para V3
3. `oauth2` crate con el endpoint de Microsoft — también válido

La arquitectura de `keep.rs` puede replicarse casi 1:1 para OneNote, solo cambiando
los endpoints de auth y el client de API.
