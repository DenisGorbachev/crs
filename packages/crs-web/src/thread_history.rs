use crate::{CodexConnection, FetchPagesError, canonical_json, content_version, fetch_pages};
use codex_app_server_protocol::{ClientRequest, RequestId, SortDirection, Thread, ThreadHistoryMode, ThreadItem, ThreadItemEntry, ThreadItemPayload, ThreadItemsListCursor, ThreadItemsListParams, ThreadItemsListResponse, ThreadTurnsListParams, ThreadTurnsListResponse, Turn, TurnItemsView};
use errgonomic::{handle, handle_bool, handle_opt};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ThreadHistory {
    pub turns: Vec<Turn>,
    pub items: Vec<ThreadItemEntry>,
}

impl ThreadHistory {
    pub fn last_turn_id(&self) -> Option<&str> {
        self.turns.last().map(|turn| turn.id.as_str())
    }

    pub fn into_visible_turns(self) -> Result<Vec<Turn>, ThreadHistoryIntoVisibleTurnsError> {
        use ThreadHistoryIntoVisibleTurnsError::*;
        let Self {
            mut turns,
            items,
        } = self;
        let mut by_id: HashMap<_, _> = turns
            .iter_mut()
            .map(|turn| (turn.id.as_str(), &mut turn.items))
            .collect();
        for entry in items {
            let dest = handle_opt!(by_id.get_mut(entry.turn_id.as_str()), TurnNotFound, id: entry.turn_id);
            dest.push(handle!(ThreadItem::try_from(entry), DeferredOutputInvalid));
        }
        for turn in &mut turns {
            turn.items
                .retain(|item| !matches!(item.payload, ThreadItemPayload::Reasoning { .. }));
        }
        Ok(turns)
    }

    pub async fn fetch(client: &CodexConnection, thread: &Thread) -> Result<Self, ThreadHistoryFetchError> {
        use ThreadHistoryFetchError::*;
        let thread_id = thread.id.as_str();
        let legacy = thread.history_mode == ThreadHistoryMode::Legacy;
        let turns = fetch_pages(|cursor| async move {
            let response: ThreadTurnsListResponse = client
                .request(ClientRequest::ThreadTurnsList {
                    request_id: RequestId::String(Uuid::new_v4().to_string()),
                    params: ThreadTurnsListParams {
                        thread_id: thread_id.to_owned(),
                        cursor,
                        limit: None,
                        sort_direction: Some(SortDirection::Asc),
                        items_view: Some(if legacy { TurnItemsView::Full } else { TurnItemsView::NotLoaded }),
                    },
                })
                .await?;
            Ok((response.data, response.next_cursor))
        })
        .await;
        let turns = handle!(turns, FetchTurnPagesFailed, thread_id);
        if legacy {
            handle_bool!(
                turns
                    .iter()
                    .any(|turn| turn.items_view != TurnItemsView::Full),
                LegacyItemsInvalid,
                thread_id
            );
            return Ok(Self {
                turns,
                items: Vec::new(),
            });
        }
        let items = fetch_pages(|cursor| async move {
            let response: ThreadItemsListResponse = client
                .request(ClientRequest::ThreadItemsList {
                    request_id: RequestId::String(Uuid::new_v4().to_string()),
                    params: ThreadItemsListParams {
                        thread_id: thread_id.to_owned(),
                        turn_id: None,
                        cursor: cursor.map(ThreadItemsListCursor::Opaque),
                        limit: None,
                        sort_direction: Some(SortDirection::Asc),
                        defer_output: None,
                    },
                })
                .await?;
            Ok((response.data, response.next_cursor))
        })
        .await;
        let items = handle!(items, FetchItemPagesFailed, thread_id);
        handle_bool!(items.iter().any(|entry| entry.output_deferred), ItemOutputInvalid, thread_id);
        let turn_ids: HashSet<_> = turns.iter().map(|turn| turn.id.as_str()).collect();
        handle_bool!(
            items
                .iter()
                .any(|entry| !turn_ids.contains(entry.turn_id.as_str())),
            ItemTurnInvalid,
            thread_id
        );
        Ok(Self {
            turns,
            items,
        })
    }
}

pub fn turn_versions(turns: &[Turn]) -> Result<Vec<String>, serde_json::Error> {
    turns
        .iter()
        .map(|turn| canonical_json(turn).map(|json| content_version(&json)))
        .collect()
}

#[derive(Error, Debug)]
pub enum ThreadHistoryIntoVisibleTurnsError {
    #[error("saved thread contains deferred item output")]
    DeferredOutputInvalid { source: codex_app_server_protocol::DeferredItemOutput },
    #[error("saved item refers to missing turn '{id}'")]
    TurnNotFound { id: String },
}

#[derive(Error, Debug)]
pub enum ThreadHistoryFetchError {
    #[error("failed to fetch turns for Codex thread '{thread_id}'")]
    FetchTurnPagesFailed { source: FetchPagesError, thread_id: String },
    #[error("failed to fetch items for Codex thread '{thread_id}'")]
    FetchItemPagesFailed { source: FetchPagesError, thread_id: String },
    #[error("Codex returned incomplete legacy turns for thread '{thread_id}'")]
    LegacyItemsInvalid { thread_id: String },
    #[error("Codex deferred item output for thread '{thread_id}'")]
    ItemOutputInvalid { thread_id: String },
    #[error("Codex history changed while downloading thread '{thread_id}'")]
    ItemTurnInvalid { thread_id: String },
}
