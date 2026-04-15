use std::num::NonZeroU32;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use tokio::sync::Mutex;
use tracing::{debug, info};

use skeepy_core::{
    ChecklistItem, CreateNoteRequest as CoreCreateRequest, Label, NoteColor, NoteContent,
    NoteProvider, ProviderCapabilities, ProviderError, ProviderStability, ProviderStatus,
    RemoteNote, UpdateNoteRequest,
};

use crate::api::{OneNoteApiClient, Page};
use crate::auth::refresh_access_token;
use crate::html::html_to_text;
use crate::token::{TokenSet, TokenStorage};

// ─── OneNoteProvider ──────────────────────────────────────────────────────────

pub struct OneNoteProvider {
    client_id: String,
    api: OneNoteApiClient,
    rate_limiter: DefaultDirectRateLimiter,
    tokens: Arc<Mutex<Option<TokenSet>>>,
    http: reqwest::Client,
}

impl OneNoteProvider {
    pub fn new(client_id: impl Into<String>) -> Self {
        // 120 requests per minute — conservative for Graph API.
        let quota = Quota::per_minute(NonZeroU32::new(120).unwrap());
        let rate_limiter = RateLimiter::direct(quota);

        Self {
            client_id: client_id.into(),
            api: OneNoteApiClient::new(),
            rate_limiter,
            tokens: Arc::new(Mutex::new(None)),
            http: reqwest::Client::new(),
        }
    }

    async fn get_access_token(&self) -> Result<String, ProviderError> {
        let mut guard = self.tokens.lock().await;

        // Try in-memory cache first.
        if let Some(ref tokens) = *guard {
            if !tokens.is_expired() {
                return Ok(tokens.access_token.clone());
            }
            if let Some(ref refresh) = tokens.refresh_token.clone() {
                let refreshed =
                    refresh_access_token(&self.http, &self.client_id, refresh).await?;
                let token = refreshed.access_token.clone();
                TokenStorage::save(&refreshed)?;
                *guard = Some(refreshed);
                return Ok(token);
            }
        }

        // Load from keyring.
        let tokens = TokenStorage::load()?.ok_or(ProviderError::AuthRequired)?;
        if tokens.is_expired() {
            let refresh = tokens.refresh_token.ok_or(ProviderError::AuthRequired)?;
            let refreshed =
                refresh_access_token(&self.http, &self.client_id, &refresh).await?;
            let token = refreshed.access_token.clone();
            TokenStorage::save(&refreshed)?;
            *guard = Some(refreshed);
            Ok(token)
        } else {
            let token = tokens.access_token.clone();
            *guard = Some(tokens);
            Ok(token)
        }
    }
}

#[async_trait]
impl NoteProvider for OneNoteProvider {
    fn id(&self) -> &str { "onenote" }

    fn display_name(&self) -> &str { "Microsoft OneNote" }

    fn status(&self) -> ProviderStatus { ProviderStatus::Active }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            can_read: true,
            can_write: true,
            can_delete: true,
            supports_labels: false,
            supports_colors: false,
            supports_checklists: false,
            supports_incremental_sync: false,
            stability: ProviderStability::Experimental,
        }
    }

    async fn authenticate(&mut self) -> Result<(), ProviderError> {
        self.get_access_token().await.map(|_| ())
    }

    async fn is_authenticated(&self) -> bool {
        self.get_access_token().await.is_ok()
    }

    async fn revoke_auth(&mut self) -> Result<(), ProviderError> {
        let mut guard = self.tokens.lock().await;
        *guard = None;
        TokenStorage::delete()
    }

    async fn fetch_notes(
        &self,
        _since: Option<DateTime<Utc>>,
    ) -> Result<Vec<RemoteNote>, ProviderError> {
        let access_token = self.get_access_token().await?;
        self.rate_limiter.until_ready().await;

        let pages = self.api.list_all_pages(&access_token).await?;
        info!(count = pages.len(), "Fetched pages from OneNote");

        let mut notes = Vec::with_capacity(pages.len());
        for (page, section_name) in pages {
            self.rate_limiter.until_ready().await;

            let html = match self.api.get_page_content(&access_token, &page.id).await {
                Ok(h) => h,
                Err(e) => {
                    debug!(page_id = %page.id, error = %e, "Skipping page — content fetch failed");
                    continue;
                }
            };

            notes.push(page_to_remote(page, section_name, html));
        }

        Ok(notes)
    }

    async fn fetch_note(&self, source_id: &str) -> Result<RemoteNote, ProviderError> {
        let access_token = self.get_access_token().await?;
        self.rate_limiter.until_ready().await;

        let html = self.api.get_page_content(&access_token, source_id).await?;
        debug!(page_id = %source_id, "Fetched single OneNote page");

        // We only have source_id here — use minimal metadata.
        let now = Utc::now();
        let remote = RemoteNote {
            source_id: source_id.to_string(),
            title: None,
            content: NoteContent::Text(html_to_text(&html)),
            labels: vec![],
            color: NoteColor::Default,
            is_pinned: false,
            is_archived: false,
            is_trashed: false,
            created_at: now,
            updated_at: now,
        };
        Ok(remote)
    }

    async fn create_note(&self, req: CoreCreateRequest) -> Result<RemoteNote, ProviderError> {
        let access_token = self.get_access_token().await?;
        self.rate_limiter.until_ready().await;

        // Pick the first available section.
        let sections = self.api.list_sections(&access_token).await?;
        let section = sections
            .into_iter()
            .next()
            .ok_or_else(|| ProviderError::Api("No sections found in OneNote".to_string()))?;

        let title = req.title.as_deref().unwrap_or("Untitled");
        let body_html = match req.content {
            NoteContent::Text(ref text) => text_to_html(text),
            NoteContent::Checklist(ref items) => checklist_to_html(items),
        };

        let page = self
            .api
            .create_page(&access_token, &section.id, title, &body_html)
            .await?;

        info!(page_id = %page.id, "Created page in OneNote");

        let section_name = section.display_name.clone();
        let html = format!("<html><body>{body_html}</body></html>");
        Ok(page_to_remote(page, section_name, html))
    }

    async fn update_note(
        &self,
        source_id: &str,
        req: UpdateNoteRequest,
    ) -> Result<RemoteNote, ProviderError> {
        let access_token = self.get_access_token().await?;
        self.rate_limiter.until_ready().await;

        let body_html = match req.content {
            NoteContent::Text(ref text) => text_to_html(text),
            NoteContent::Checklist(ref items) => checklist_to_html(items),
        };

        self.api
            .update_page(
                &access_token,
                source_id,
                req.title.as_deref(),
                &body_html,
            )
            .await?;

        info!(page_id = %source_id, "Updated page in OneNote");

        // Graph PATCH returns 204 No Content, so we reconstruct the RemoteNote.
        let now = Utc::now();
        let text = match req.content {
            NoteContent::Text(ref t) => t.clone(),
            NoteContent::Checklist(ref items) => {
                items.iter().map(|i| i.text.as_str()).collect::<Vec<_>>().join("\n")
            }
        };

        Ok(RemoteNote {
            source_id: source_id.to_string(),
            title: req.title,
            content: NoteContent::Text(text),
            labels: vec![],
            color: NoteColor::Default,
            is_pinned: false,
            is_archived: false,
            is_trashed: false,
            created_at: now,
            updated_at: now,
        })
    }

    async fn delete_note(&self, source_id: &str) -> Result<(), ProviderError> {
        let access_token = self.get_access_token().await?;
        self.rate_limiter.until_ready().await;

        self.api.delete_page(&access_token, source_id).await?;
        info!(page_id = %source_id, "Deleted page from OneNote");
        Ok(())
    }
}

// ─── Mapping helpers ──────────────────────────────────────────────────────────

fn page_to_remote(page: Page, section_name: String, html: String) -> RemoteNote {
    let text = html_to_text(&html);

    let created_at = parse_ms_datetime(&page.created_date_time).unwrap_or_else(Utc::now);
    let updated_at = parse_ms_datetime(&page.last_modified_date_time).unwrap_or_else(Utc::now);

    // Use section name as a label for discoverability.
    let labels = if !section_name.is_empty() {
        vec![Label { id: section_name.clone(), name: section_name }]
    } else {
        vec![]
    };

    RemoteNote {
        source_id: page.id,
        title: if page.title.is_empty() { None } else { Some(page.title) },
        content: NoteContent::Text(text),
        labels,
        color: NoteColor::Default,
        is_pinned: false,
        is_archived: false,
        is_trashed: false,
        created_at,
        updated_at,
    }
}

fn parse_ms_datetime(s: &str) -> Option<DateTime<Utc>> {
    // Microsoft Graph returns ISO 8601 / RFC 3339 strings.
    DateTime::parse_from_rfc3339(s).ok().map(Into::into)
}

fn text_to_html(text: &str) -> String {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    // Wrap each line in a <p> tag.
    escaped
        .lines()
        .map(|line| format!("<p>{line}</p>"))
        .collect::<Vec<_>>()
        .join("")
}

fn checklist_to_html(items: &[ChecklistItem]) -> String {
    let lis: String = items
        .iter()
        .map(|item| {
            let checked = if item.checked { " checked" } else { "" };
            format!(
                "<li><input type=\"checkbox\"{checked}/> {}</li>",
                item.text
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
            )
        })
        .collect();
    format!("<ul>{lis}</ul>")
}
