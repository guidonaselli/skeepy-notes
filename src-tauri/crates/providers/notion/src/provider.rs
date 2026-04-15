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

use crate::api::{blocks_to_text, NotionApiClient, NotionPage};
use crate::token::{TokenSet, TokenStorage};

// ─── NotionProvider ───────────────────────────────────────────────────────────

pub struct NotionProvider {
    client_id: String,
    client_secret: String,
    api: NotionApiClient,
    rate_limiter: DefaultDirectRateLimiter,
    tokens: Arc<Mutex<Option<TokenSet>>>,
    /// Default parent page ID for creating new notes.
    default_parent_id: Option<String>,
}

impl NotionProvider {
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        default_parent_id: Option<String>,
    ) -> Self {
        // Notion rate limit: ~3 requests/second per integration.
        let quota = Quota::per_second(std::num::NonZeroU32::new(3).unwrap());
        let rate_limiter = RateLimiter::direct(quota);

        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            api: NotionApiClient::new(),
            rate_limiter,
            tokens: Arc::new(Mutex::new(None)),
            default_parent_id,
        }
    }

    async fn get_access_token(&self) -> Result<String, ProviderError> {
        let mut guard = self.tokens.lock().await;

        if let Some(ref tokens) = *guard {
            return Ok(tokens.access_token.clone());
        }

        // Load from keyring.
        let tokens = TokenStorage::load()?.ok_or(ProviderError::AuthRequired)?;
        let token = tokens.access_token.clone();
        *guard = Some(tokens);
        Ok(token)
    }
}

#[async_trait]
impl NoteProvider for NotionProvider {
    fn id(&self) -> &str { "notion" }

    fn display_name(&self) -> &str { "Notion" }

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

        let pages = self.api.list_pages(&access_token).await?;
        info!(count = pages.len(), "Fetched pages from Notion");

        let mut notes = Vec::with_capacity(pages.len());
        for page in pages {
            if page.archived { continue; }

            self.rate_limiter.until_ready().await;
            let blocks = match self.api.get_page_blocks(&access_token, &page.id).await {
                Ok(b) => b,
                Err(e) => {
                    debug!(page_id = %page.id, error = %e, "Skipping page — blocks fetch failed");
                    continue;
                }
            };

            notes.push(page_to_remote(page, blocks));
        }

        Ok(notes)
    }

    async fn fetch_note(&self, source_id: &str) -> Result<RemoteNote, ProviderError> {
        let access_token = self.get_access_token().await?;
        self.rate_limiter.until_ready().await;

        let blocks = self.api.get_page_blocks(&access_token, source_id).await?;
        let text = blocks_to_text(&blocks);

        let now = Utc::now();
        Ok(RemoteNote {
            source_id: source_id.to_string(),
            title: None,
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

    async fn create_note(&self, req: CoreCreateRequest) -> Result<RemoteNote, ProviderError> {
        let access_token = self.get_access_token().await?;
        self.rate_limiter.until_ready().await;

        let parent_id = self
            .default_parent_id
            .as_deref()
            .ok_or_else(|| ProviderError::Api(
                "No default parent page configured for Notion. Set a parent page ID in Settings.".to_string()
            ))?;

        let title = req.title.as_deref().unwrap_or("Untitled");
        let content = content_to_text(&req.content);

        let page = self.api.create_page(&access_token, parent_id, title, &content).await?;
        info!(page_id = %page.id, "Created page in Notion");

        Ok(page_to_remote(page, vec![]))
    }

    async fn update_note(
        &self,
        source_id: &str,
        req: UpdateNoteRequest,
    ) -> Result<RemoteNote, ProviderError> {
        let access_token = self.get_access_token().await?;
        self.rate_limiter.until_ready().await;

        let content = content_to_text(&req.content);
        self.api
            .update_page(&access_token, source_id, req.title.as_deref(), &content)
            .await?;

        info!(page_id = %source_id, "Updated page in Notion");

        let now = Utc::now();
        Ok(RemoteNote {
            source_id: source_id.to_string(),
            title: req.title,
            content: req.content,
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

        self.api.archive_page(&access_token, source_id).await?;
        info!(page_id = %source_id, "Archived page in Notion");
        Ok(())
    }
}

// ─── Mapping ──────────────────────────────────────────────────────────────────

fn page_to_remote(page: NotionPage, blocks: Vec<crate::api::Block>) -> RemoteNote {
    let text = blocks_to_text(&blocks);
    let title = page.title();

    let created_at = DateTime::parse_from_rfc3339(&page.created_time)
        .ok()
        .map(Into::into)
        .unwrap_or_else(Utc::now);

    let updated_at = DateTime::parse_from_rfc3339(&page.last_edited_time)
        .ok()
        .map(Into::into)
        .unwrap_or_else(Utc::now);

    RemoteNote {
        source_id: page.id,
        title,
        content: NoteContent::Text(text),
        labels: vec![],
        color: NoteColor::Default,
        is_pinned: false,
        is_archived: page.archived,
        is_trashed: false,
        created_at,
        updated_at,
    }
}

fn content_to_text(content: &NoteContent) -> String {
    match content {
        NoteContent::Text(t) => t.clone(),
        NoteContent::Checklist(items) => items
            .iter()
            .map(|i| format!("{} {}", if i.checked { "[x]" } else { "[ ]" }, i.text))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}
