# S12: Google OAuth Verification

**Goal:** Iniciar el proceso formal de verificación de la app en Google para que los usuarios
no vean la pantalla "Esta app no está verificada".

**Estado actual:** Sin verificación, Google muestra "⚠ Google no verificó esta app".
Los usuarios deben hacer click en "Avanzado → Continuar" — fricción que hace que muchos abandonen.

## Must-Haves

- Privacy policy publicada en una URL pública
- Dominio verificado en Google Search Console
- Formulario de verificación OAuth enviado a Google
- Documentado en DECISIONS.md con el estado y la fecha de envío

## Out of Scope

- Video demo (solo requerido si Google lo solicita en el review)
- Dominio propio completo (puede ser GitHub Pages para V1)

## Timeline esperado

4-6 semanas desde el envío del formulario. NO es un blocker para el release — la app
funciona igual, solo muestra la advertencia.

## Tasks

- [ ] **T01: Privacy Policy**
  Crear una página simple (GitHub Pages o similar) con la política de privacidad.
  Contenido mínimo requerido por Google:
  - Qué datos se recopilan (ninguno — todo local)
  - Cómo se usan los scopes de Google (keep.readonly = solo lectura)
  - Cómo el usuario puede revocar el acceso
  - Datos de contacto del desarrollador

- [ ] **T02: Verificar dominio en Google Search Console**
  Si se usa GitHub Pages: agregar el meta tag de verificación al `<head>` del sitio.
  Alternativamente usar verificación DNS si se tiene un dominio propio.

- [ ] **T03: Completar el formulario en Google Cloud Console**
  En Google Cloud Console → OAuth consent screen:
  - App name: "Skeepy Notes"
  - App logo
  - Privacy policy URL
  - Authorized domains
  - Descripción clara de por qué se necesita `keep.readonly`
  - Hacer click en "Submit for verification"

- [ ] **T04: Documentar en DECISIONS.md**
  Agregar fila:
  `| D020 | M002 | compliance | Google OAuth Verification | Enviado (fecha) — en review | ... |`

## Notes

Mientras la verificación está en proceso, el flujo de OAuth sigue funcionando para
los test users (hasta 100 usuarios agregados manualmente en Google Cloud Console).
Para el lanzamiento público inicial, agregar manualmente a los primeros usuarios como
"test users" hasta que llegue la verificación.
