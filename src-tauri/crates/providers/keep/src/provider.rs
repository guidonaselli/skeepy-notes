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
    RemoteNote,
};

use crate::api::{
    CreateListBody, CreateListItem, CreateNoteBody, CreateNoteRequest, CreateTextBody,
    KeepApiClient, KeepNote, ListItem,
};
use crate::auth::refresh_access_token;
use crate::token::{TokenSet, TokenStorage};

// ─── KeepProvider ─────────────────────────────────────────────────────────────

pub struct KeepProvider {
    client_id: String,
    client_secret: Option<String>,
    api: KeepApiClient,
    rate_limiter: DefaultDirectRateLimiter,
    /// Cached tokens — updated after refresh.
    tokens: Arc<Mutex<Option<TokenSet>>>,
    http: reqwest::Client,
}

impl KeepProvider {
    /// Construct a new provider.
    ///
    /// `client_id` and (optionally) `client_secret` come from the user's
    /// Google Cloud Console project. Tokens are loaded from keyring on first use.
    pub fn new(client_id: impl Into<String>, client_secret: Option<String>) -> Self {
        // 30 requests per minute — well within Google Keep API limits.
        let quota = Quota::per_minute(NonZeroU32::new(30).unwrap());
        let rate_limiter = RateLimiter::direct(quota);

        Self {
            client_id: client_id.into(),
            client_secret,
            api: KeepApiClient::new(),
            rate_limiter,
            tokens: Arc::new(Mutex::new(None)),
            http: reqwest::Client::new(),
        }
    }

    /// Get a valid access token, refreshing if necessary.
    async fn get_access_token(&self) -> Result<String, ProviderError> {
        let mut guard = self.tokens.lock().await;

        // Try in-memory cache first
        if let Some(ref tokens) = *guard {
            if !tokens.is_expired() {
                return Ok(tokens.access_token.clone());
            }
            // Try refreshing
            if let Some(ref refresh) = tokens.refresh_token.clone() {
                let refreshed = refresh_access_token(
                    &self.http,
                    &self.client_id,
                    self.client_secret.as_deref(),
                    refresh,
                )
                .await?;
                let token = refreshed.access_token.clone();
                TokenStorage::save(&refreshed)?;
                *guard = Some(refreshed);
                return Ok(token);
            }
        }

        // Load from keyring
        let tokens = TokenStorage::load()?.ok_or(ProviderError::AuthRequired)?;
        if tokens.is_expired() {
            let refresh = tokens.refresh_token.ok_or(ProviderError::AuthRequired)?;
            let refreshed = refresh_access_token(
                &self.http,
                &self.client_id,
                self.client_secret.as_deref(),
                &refresh,
            )
            .await?;
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
impl NoteProvider for KeepProvider {
    fn id(&self) -> &str { "keep" }

    fn display_name(&self) -> &str { "Google Keep" }

    fn status(&self) -> ProviderStatus { ProviderStatus::Active }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            can_read: true,
            can_write: true,   // POST /notes supported
            can_delete: true,  // DELETE /notes/{name} supported
            supports_labels: true,
            supports_colors: true,
            supports_checklists: true,
            supports_incremental_sync: true,
            stability: ProviderStability::Experimental,
        }
    }

    async fn authenticate(&mut self) -> Result<(), ProviderError> {
        // Auth is done via the IPC `keep_complete_auth` command — tokens already in keyring.
        // This just verifies we can get a valid access token.
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
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<RemoteNote>, ProviderError> {
        let access_token = self.get_access_token().await?;

        self.rate_limiter.until_ready().await;

        let keep_notes = self.api.list_all_notes(&access_token, since.as_ref()).await?;

        info!(count = keep_notes.len(), "Fetched notes from Keep");

        let notes = keep_notes.into_iter().map(keep_note_to_remote).collect();
        Ok(notes)
    }

    async fn fetch_note(&self, source_id: &str) -> Result<RemoteNote, ProviderError> {
        let access_token = self.get_access_token().await?;

        self.rate_limiter.until_ready().await;

        let note = self.api.get_note(&access_token, source_id).await?;
        debug!(name = %note.name, "Fetched single note from Keep");
        Ok(keep_note_to_remote(note))
    }

    async fn create_note(&self, req: CoreCreateRequest) -> Result<RemoteNote, ProviderError> {
        let access_token = self.get_access_token().await?;
        self.rate_limiter.until_ready().await;

        let api_body = match req.content {
            NoteContent::Text(text) => CreateNoteBody {
                text: Some(CreateTextBody { text }),
                list: None,
            },
            NoteContent::Checklist(items) => CreateNoteBody {
                text: None,
                list: Some(CreateListBody {
                    list_items: items
                        .into_iter()
                        .map(|i| CreateListItem {
                            text: CreateTextBody { text: i.text },
                            checked: i.checked,
                        })
                        .collect(),
                }),
            },
        };

        let api_req = CreateNoteRequest {
            title: req.title,
            body: api_body,
        };

        let created = self.api.create_note(&access_token, api_req).await?;
        info!(name = %created.name, "Created note in Keep");
        Ok(keep_note_to_remote(created))
    }

    async fn delete_note(&self, source_id: &str) -> Result<(), ProviderError> {
        let access_token = self.get_access_token().await?;
        self.rate_limiter.until_ready().await;

        self.api.delete_note(&access_token, source_id).await?;
        info!(name = %source_id, "Deleted note from Keep");
        Ok(())
    }
}

// ─── Mapping ──────────────────────────────────────────────────────────────────

fn keep_note_to_remote(note: KeepNote) -> RemoteNote {
    let source_id = note.name.clone();

    let content = match note.body {
        Some(body) if body.list.is_some() => {
            let list = body.list.unwrap();
            let items = flatten_list_items(&list.list_items);
            NoteContent::Checklist(items)
        }
        Some(body) => NoteContent::Text(body.text.map(|t| t.text).unwrap_or_default()),
        None => NoteContent::Text(String::new()),
    };

    let labels = note.labels.into_iter().map(|l| {
        // label name looks like "labelGroups/xxx/labels/yyy" — use last segment.
        let short = l.name.rsplit('/').next().unwrap_or(&l.name).to_string();
        Label { id: l.name, name: short }
    }).collect();

    let created_at = parse_rfc3339(&note.create_time).unwrap_or_else(Utc::now);
    let updated_at = parse_rfc3339(&note.update_time).unwrap_or_else(Utc::now);

    RemoteNote {
        source_id,
        title: note.title.filter(|t| !t.is_empty()),
        content,
        labels,
        color: map_keep_color(&note.color),
        is_pinned: note.starred,
        is_archived: note.archived,
        is_trashed: note.trashed,
        created_at,
        updated_at,
    }
}

fn flatten_list_items(items: &[ListItem]) -> Vec<ChecklistItem> {
    let mut result = Vec::new();
    for item in items {
        if let Some(ref text) = item.text {
            result.push(ChecklistItem { text: text.text.clone(), checked: item.checked });
        }
        // Include nested items (flatten one level)
        for child in &item.child_items {
            if let Some(ref text) = child.text {
                result.push(ChecklistItem { text: format!("  {}", text.text), checked: child.checked });
            }
        }
    }
    result
}

fn map_keep_color(color: &str) -> NoteColor {
    match color.to_uppercase().as_str() {
        "RED"       => NoteColor::Red,
        "ORANGE"    => NoteColor::Orange,
        "YELLOW"    => NoteColor::Yellow,
        "GREEN"     => NoteColor::Green,
        "TEAL"      => NoteColor::Teal,
        "CYAN"      => NoteColor::Teal, // no direct equivalent
        "BLUE"      => NoteColor::Blue,
        "DARKBLUE"  => NoteColor::DarkBlue,
        "PURPLE"    => NoteColor::Purple,
        "PINK"      => NoteColor::Pink,
        "BROWN"     => NoteColor::Brown,
        "GRAY" | "GREY" => NoteColor::Gray,
        _ => NoteColor::Default,
    }
}

fn parse_rfc3339(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(Into::into)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{NoteBody, NoteLabel, TextContent};

    fn make_note(color: &str, starred: bool, body: NoteBody) -> KeepNote {
        KeepNote {
            name: "notes/test-id".to_string(),
            create_time: "2024-01-01T00:00:00Z".to_string(),
            update_time: "2024-06-01T12:00:00Z".to_string(),
            trash_time: None,
            trashed: false,
            title: Some("Test".to_string()),
            body: Some(body),
            labels: vec![],
            color: color.to_string(),
            starred,
            archived: false,
        }
    }

    #[test]
    fn text_body_maps_correctly() {
        let body = NoteBody {
            text: Some(TextContent { text: "Hello".to_string() }),
            list: None,
        };
        let remote = keep_note_to_remote(make_note("DEFAULT", false, body));
        assert_eq!(remote.source_id, "notes/test-id");
        match remote.content {
            NoteContent::Text(t) => assert_eq!(t, "Hello"),
            _ => panic!("expected Text"),
        }
        assert!(!remote.is_pinned);
    }

    #[test]
    fn pinned_maps_from_starred() {
        let body = NoteBody { text: Some(TextContent { text: "".into() }), list: None };
        let remote = keep_note_to_remote(make_note("DEFAULT", true, body));
        assert!(remote.is_pinned);
    }

    #[test]
    fn color_mapping_is_exhaustive() {
        let cases = [
            ("RED", NoteColor::Red),
            ("ORANGE", NoteColor::Orange),
            ("YELLOW", NoteColor::Yellow),
            ("GREEN", NoteColor::Green),
            ("TEAL", NoteColor::Teal),
            ("CYAN", NoteColor::Teal),
            ("BLUE", NoteColor::Blue),
            ("PURPLE", NoteColor::Purple),
            ("PINK", NoteColor::Pink),
            ("BROWN", NoteColor::Brown),
            ("GRAY", NoteColor::Gray),
            ("DEFAULT", NoteColor::Default),
            ("UNKNOWN_COLOR", NoteColor::Default),
        ];
        for (input, expected) in cases {
            let got = map_keep_color(input);
            assert_eq!(format!("{:?}", got), format!("{:?}", expected), "Failed for {input}");
        }
    }

    #[test]
    fn label_short_name_extracted() {
        let note = KeepNote {
            name: "notes/x".to_string(),
            create_time: "2024-01-01T00:00:00Z".to_string(),
            update_time: "2024-01-01T00:00:00Z".to_string(),
            trash_time: None, trashed: false,
            title: None,
            body: None,
            labels: vec![NoteLabel { name: "labelGroups/abc/labels/my-label".to_string() }],
            color: "DEFAULT".to_string(),
            starred: false,
            archived: false,
        };
        let remote = keep_note_to_remote(note);
        assert_eq!(remote.labels[0].name, "my-label");
        assert_eq!(remote.labels[0].id, "labelGroups/abc/labels/my-label");
    }
}
