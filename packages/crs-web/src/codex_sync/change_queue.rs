use crate::{CodexConnection, CodexConnectionRequestError, CodexSync, FetchPagesError, MessageAction, PendingSteer, QueuedMessage, ThreadStoreSaveSteerError, ThreadStoreSteeringInProgressError, edit_texts};
use axum::http::StatusCode;
use codex_app_server_protocol::{ClientRequest, RequestId, SortDirection, ThreadQueueAddParams, ThreadQueueAddResponse, ThreadQueueDeleteParams, ThreadQueueDeleteResponse, ThreadQueueUpdateParams, ThreadQueueUpdateResponse, ThreadTurnsListParams, ThreadTurnsListResponse, TurnItemsView, TurnStatus, TurnSteerParams, TurnSteerResponse, UserInput};
use errgonomic::{handle, handle_bool, handle_opt};
use thiserror::Error;
use tokio::task::{JoinError, spawn_blocking};
use uuid::Uuid;

impl CodexSync {
    pub async fn change_queue(&self, client: &CodexConnection, thread_id: &str, queued_id: String, action: MessageAction, original: Option<&str>, message: String) -> Result<Option<QueuedMessage>, CodexSyncChangeQueueError> {
        use CodexSyncChangeQueueError::*;
        use MessageAction::*;
        let request_id = || RequestId::String(Uuid::new_v4().to_string());
        let delete = || {
            client.request::<ThreadQueueDeleteResponse>(ClientRequest::ThreadQueueDelete {
                request_id: request_id(),
                params: ThreadQueueDeleteParams {
                    thread_id: thread_id.into(),
                    queued_submission_id: queued_id.clone(),
                },
            })
        };
        let queue = handle!(client.queued_submissions(thread_id).await, ListFailed);
        let mut item = handle_opt!(queue.into_iter().find(|item| item.id == queued_id), AlreadyDelivered);
        let store = self.store.clone();
        let source = self.server_id.clone();
        let thread = thread_id.to_owned();
        let client_id = item.client_user_message_id.clone();
        let entry_id = queued_id.clone();
        let pending = handle!(spawn_blocking(move || store.steering_in_progress(&source, &thread, &client_id, &entry_id)).await, JoinFailed);
        handle_bool!(handle!(pending, ReadSteerFailed), SteeringInProgress);
        if matches!(action, Cancel) {
            handle_bool!(!handle!(delete().await, DeleteFailed).deleted, AlreadyDelivered);
            return Ok(None);
        }
        if matches!(action, Edit) {
            let before: Vec<String> = handle!(serde_json::from_str(original.unwrap_or_default()), InvalidEdit);
            let after: Vec<String> = handle!(serde_json::from_str(&message), InvalidEdit);
            let texts: Vec<_> = edit_texts(&item.input).collect();
            handle_bool!(texts.iter().copied().ne(before.iter().map(String::as_str)) || after.len() != before.len().max(1), EditConflict);
            handle_bool!(texts.len() == item.input.len() && after.iter().all(|part| part.trim().is_empty()), EmptyEdit);
            let mut edited = after.into_iter();
            for part in &mut item.input {
                if let UserInput::Text {
                    text,
                    text_elements,
                } = part
                    && text_elements.is_empty()
                {
                    *text = handle_opt!(edited.next(), EditConflict);
                }
            }
            if let Some(text) = edited.next()
                && !text.trim().is_empty()
            {
                item.input.push(UserInput::Text {
                    text,
                    text_elements: Vec::new(),
                });
            }
            let ack = handle!(
                client
                    .request::<ThreadQueueUpdateResponse>(ClientRequest::ThreadQueueUpdate {
                        request_id: request_id(),
                        params: ThreadQueueUpdateParams {
                            thread_id: thread_id.into(),
                            queued_submission_id: queued_id,
                            input: item.input,
                            responsesapi_client_metadata: None
                        },
                    })
                    .await,
                EditFailed
            );
            return Ok(Some(QueuedMessage::from(&ack.queued_submission)));
        }
        let turns = handle!(
            client
                .request::<ThreadTurnsListResponse>(ClientRequest::ThreadTurnsList {
                    request_id: request_id(),
                    params: ThreadTurnsListParams {
                        thread_id: thread_id.into(),
                        cursor: None,
                        limit: Some(1),
                        sort_direction: Some(SortDirection::Desc),
                        items_view: Some(TurnItemsView::Summary)
                    },
                })
                .await,
            ReadTurnFailed
        );
        let turn = handle_opt!(
            turns
                .data
                .into_iter()
                .next()
                .filter(|turn| turn.status == TurnStatus::InProgress),
            NoActiveTurn
        );
        let client_id = item.client_user_message_id.clone();
        let persist = async |record: Option<PendingSteer>| {
            let store = self.store.clone();
            let source = self.server_id.clone();
            let thread_id = thread_id.to_owned();
            let id = client_id.clone();
            let result = handle!(spawn_blocking(move || store.save_steer(&source, &thread_id, &id, record.as_ref())).await, JoinFailed);
            handle!(result, SaveSteerFailed);
            Ok(())
        };
        let mut record = PendingSteer {
            item,
            turn_id: turn.id,
            confirmed: false,
        };
        persist(Some(record.clone())).await?;
        match delete().await {
            Ok(response) if response.deleted => (),
            Ok(_) => {
                persist(None).await?;
                return Err(AlreadyDelivered {});
            }
            Err(source) => {
                if source.rejected_before_admission() {
                    persist(None).await?;
                }
                return Err(DeleteFailed {
                    source,
                });
            }
        }
        let result = client
            .request::<TurnSteerResponse>(ClientRequest::TurnSteer {
                request_id: request_id(),
                params: TurnSteerParams {
                    thread_id: thread_id.into(),
                    expected_turn_id: record.turn_id.clone(),
                    client_user_message_id: Some(client_id.clone()),
                    input: record.item.input.clone(),
                    responsesapi_client_metadata: record.item.responsesapi_client_metadata.clone(),
                    responsesapi_client_metadata_patch: None,
                    additional_context: None,
                },
            })
            .await;
        if let Err(source) = result {
            if source.rejected_before_admission() {
                handle!(
                    client
                        .request::<ThreadQueueAddResponse>(ClientRequest::ThreadQueueAdd {
                            request_id: request_id(),
                            params: ThreadQueueAddParams {
                                thread_id: thread_id.into(),
                                client_user_message_id: client_id.clone(),
                                input: record.item.input,
                                responsesapi_client_metadata: record.item.responsesapi_client_metadata
                            },
                        })
                        .await,
                    RestoreFailed
                );
                persist(None).await?;
                return Err(SteerRejected {
                    source,
                });
            }
            return Err(SteerUnconfirmed {
                source,
            });
        }
        record.confirmed = true;
        let preview = handle!(QueuedMessage::try_from(&record), PreviewFailed);
        persist(Some(record)).await?;
        Ok(Some(preview))
    }
}

#[derive(Error, Debug)]
pub enum CodexSyncChangeQueueError {
    #[error("Could not encode the accepted steering message.")]
    PreviewFailed { source: serde_json::Error },
    #[error("Could not read the pending steering operation.")]
    ReadSteerFailed { source: ThreadStoreSteeringInProgressError },
    #[error("Steering delivery is still being reconciled.")]
    SteeringInProgress {},
    #[error("Invalid queued edit.")]
    InvalidEdit { source: serde_json::Error },
    #[error("Enter a message before saving.")]
    EmptyEdit {},
    #[error("steering recovery task failed")]
    JoinFailed { source: JoinError },
    #[error("could not persist steering recovery")]
    SaveSteerFailed { source: ThreadStoreSaveSteerError },
    #[error("Could not read the Codex queue.")]
    ListFailed { source: FetchPagesError },
    #[error("This message is no longer in the queue.")]
    AlreadyDelivered {},
    #[error("The queued input changed. Reopen Edit to see its current text.")]
    EditConflict {},
    #[error("Could not save the queued edit.")]
    EditFailed { source: CodexConnectionRequestError },
    #[error("Could not confirm removal of the queued message.")]
    DeleteFailed { source: CodexConnectionRequestError },
    #[error("Could not read the active turn.")]
    ReadTurnFailed { source: CodexConnectionRequestError },
    #[error("There is no active turn to steer.")]
    NoActiveTurn {},
    #[error("Codex declined the steer. The message was returned to the queue.")]
    SteerRejected { source: CodexConnectionRequestError },
    #[error("Steering delivery is unconfirmed. Check the thread before resending.")]
    SteerUnconfirmed { source: CodexConnectionRequestError },
    #[error("Codex declined the steer and did not confirm that the message was requeued. Check the thread before resending.")]
    RestoreFailed { source: CodexConnectionRequestError },
}

impl CodexSyncChangeQueueError {
    pub fn status(&self) -> StatusCode {
        use CodexSyncChangeQueueError::*;
        match self {
            InvalidEdit {
                ..
            }
            | EmptyEdit {} => StatusCode::UNPROCESSABLE_ENTITY,
            AlreadyDelivered {}
            | EditConflict {}
            | SteeringInProgress {}
            | NoActiveTurn {}
            | SteerRejected {
                ..
            } => StatusCode::CONFLICT,
            _ => StatusCode::BAD_GATEWAY,
        }
    }
}
