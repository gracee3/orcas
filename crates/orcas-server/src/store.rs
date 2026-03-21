use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use orcas_core::ipc::{
    OperatorInboxChange, OperatorInboxCheckpoint, OperatorInboxItem, OperatorInboxMirrorCheckpoint,
    OperatorInboxMirrorListResponse, OperatorNotificationAckRequest,
    OperatorNotificationAckResponse, OperatorNotificationCandidate,
    OperatorNotificationCandidateStatus, OperatorNotificationGetRequest,
    OperatorNotificationListRequest, OperatorNotificationListResponse,
    OperatorNotificationSuppressRequest, OperatorNotificationSuppressResponse,
};
use orcas_core::{OrcasError, OrcasResult};
use rusqlite::{Connection, OptionalExtension, params};

const INITIAL_SCHEMA: &str = r#"
create table if not exists mirrored_inbox_items (
  origin_node_id text not null,
  item_id text not null,
  sequence integer not null,
  item_json text not null,
  changed_at text not null,
  primary key (origin_node_id, item_id)
);

create table if not exists mirrored_inbox_checkpoint (
  origin_node_id text primary key,
  current_sequence integer not null,
  updated_at text not null
);

create table if not exists mirrored_notification_candidates (
  candidate_id text primary key,
  origin_node_id text not null,
  item_id text not null,
  trigger_sequence integer not null,
  candidate_status text not null,
  item_json text not null,
  created_at text not null,
  updated_at text not null,
  acknowledged_at text,
  suppressed_at text,
  resolved_at text,
  obsolete_at text
);

create index if not exists mirrored_notification_candidates_origin_idx
  on mirrored_notification_candidates(origin_node_id, candidate_status, updated_at desc);

create table if not exists mirrored_notification_windows (
  origin_node_id text not null,
  item_id text not null,
  candidate_id text not null,
  opened_sequence integer not null,
  updated_sequence integer not null,
  updated_at text not null,
  primary key (origin_node_id, item_id)
);
"#;

fn db_error(error: rusqlite::Error) -> OrcasError {
    OrcasError::Store(error.to_string())
}

#[derive(Debug)]
pub struct InboxMirrorStore {
    connection: Mutex<Connection>,
}

#[derive(Debug, Clone)]
pub struct MirrorApplyResult {
    pub checkpoint: OperatorInboxCheckpoint,
    pub mirror_checkpoint: OperatorInboxMirrorCheckpoint,
    pub applied_changes: usize,
    pub skipped_changes: usize,
}

impl InboxMirrorStore {
    pub fn open(path: impl AsRef<Path>) -> OrcasResult<Self> {
        let connection = Connection::open(path).map_err(db_error)?;
        connection.execute_batch(INITIAL_SCHEMA).map_err(db_error)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn checkpoint(&self, origin_node_id: &str) -> OrcasResult<OperatorInboxCheckpoint> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| OrcasError::Store("mirror store connection lock poisoned".to_string()))?;
        let mut statement = connection.prepare(
            "select current_sequence, updated_at from mirrored_inbox_checkpoint where origin_node_id = ?1",
        ).map_err(db_error)?;
        let checkpoint = statement
            .query_row(params![origin_node_id], |row| {
                Ok(OperatorInboxCheckpoint {
                    current_sequence: row.get::<_, i64>(0)? as u64,
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .map(|value| value.with_timezone(&Utc))
                        .map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                1,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?,
                })
            })
            .optional()
            .map_err(db_error)?;
        Ok(checkpoint.unwrap_or_default())
    }

    pub fn list(
        &self,
        origin_node_id: &str,
        limit: Option<usize>,
    ) -> OrcasResult<OperatorInboxMirrorListResponse> {
        let checkpoint = self.checkpoint(origin_node_id)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| OrcasError::Store("mirror store connection lock poisoned".to_string()))?;
        let mut statement = connection.prepare(
            "select item_json from mirrored_inbox_items where origin_node_id = ?1 order by changed_at desc, item_id asc",
        ).map_err(db_error)?;
        let mut rows = statement.query(params![origin_node_id]).map_err(db_error)?;
        let mut items = Vec::new();
        while let Some(row) = rows.next().map_err(db_error)? {
            let item_json = row.get::<_, String>(0).map_err(db_error)?;
            let item: OperatorInboxItem = serde_json::from_str(&item_json)?;
            items.push(item);
        }
        if let Some(limit) = limit {
            items.truncate(limit);
        }
        Ok(OperatorInboxMirrorListResponse {
            origin_node_id: origin_node_id.to_string(),
            checkpoint,
            items,
        })
    }

    pub fn get(
        &self,
        origin_node_id: &str,
        item_id: &str,
    ) -> OrcasResult<Option<OperatorInboxItem>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| OrcasError::Store("mirror store connection lock poisoned".to_string()))?;
        let mut statement = connection.prepare(
            "select item_json from mirrored_inbox_items where origin_node_id = ?1 and item_id = ?2",
        ).map_err(db_error)?;
        let item = statement
            .query_row(params![origin_node_id, item_id], |row| {
                let item_json = row.get::<_, String>(0)?;
                serde_json::from_str::<OperatorInboxItem>(&item_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })
            })
            .optional()
            .map_err(db_error)?;
        Ok(item)
    }

    pub fn apply_batch(
        &self,
        origin_node_id: &str,
        _source_checkpoint: OperatorInboxCheckpoint,
        changes: &[OperatorInboxChange],
    ) -> OrcasResult<MirrorApplyResult> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| OrcasError::Store("mirror store connection lock poisoned".to_string()))?;
        let transaction = connection.transaction().map_err(db_error)?;
        let mut checkpoint = self.load_checkpoint_tx(&transaction, origin_node_id)?;
        let mut applied_changes = 0usize;
        let mut skipped_changes = 0usize;

        for change in changes {
            if change.sequence <= checkpoint.current_sequence {
                skipped_changes += 1;
                continue;
            }
            let expected = checkpoint.current_sequence + 1;
            if change.sequence != expected {
                return Err(OrcasError::Store(format!(
                    "inbox mirror batch for origin `{origin_node_id}` is missing sequence {expected} before {}",
                    change.sequence
                )));
            }
            let previous_item =
                Self::load_mirrored_item_tx(&transaction, origin_node_id, change.item.id.as_str())?;
            match change.kind {
                orcas_core::ipc::OperatorInboxChangeKind::Upsert => {
                    let item_json = serde_json::to_string(&change.item)?;
                    transaction.execute(
                        "insert into mirrored_inbox_items(origin_node_id, item_id, sequence, item_json, changed_at)
                         values(?1, ?2, ?3, ?4, ?5)
                         on conflict(origin_node_id, item_id) do update set
                           sequence = excluded.sequence,
                           item_json = excluded.item_json,
                           changed_at = excluded.changed_at",
                        params![
                            origin_node_id,
                            change.item.id.as_str(),
                            change.sequence as i64,
                            item_json,
                            change.changed_at.to_rfc3339(),
                        ],
                    ).map_err(db_error)?;
                    self.apply_notification_transition_tx(
                        &transaction,
                        origin_node_id,
                        previous_item.as_ref(),
                        Some(&change.item),
                        change.sequence,
                        change.changed_at,
                    )?;
                }
                orcas_core::ipc::OperatorInboxChangeKind::Removed => {
                    transaction.execute(
                        "delete from mirrored_inbox_items where origin_node_id = ?1 and item_id = ?2",
                        params![origin_node_id, change.item.id.as_str()],
                    ).map_err(db_error)?;
                    self.apply_notification_transition_tx(
                        &transaction,
                        origin_node_id,
                        previous_item.as_ref(),
                        None,
                        change.sequence,
                        change.changed_at,
                    )?;
                }
            }
            checkpoint.current_sequence = change.sequence;
            checkpoint.updated_at = change.changed_at;
            applied_changes += 1;
        }

        transaction.execute(
            "insert into mirrored_inbox_checkpoint(origin_node_id, current_sequence, updated_at)
             values(?1, ?2, ?3)
             on conflict(origin_node_id) do update set
               current_sequence = excluded.current_sequence,
               updated_at = excluded.updated_at",
            params![
                origin_node_id,
                checkpoint.current_sequence as i64,
                checkpoint.updated_at.to_rfc3339(),
            ],
        ).map_err(db_error)?;
        transaction.commit().map_err(db_error)?;

        Ok(MirrorApplyResult {
            mirror_checkpoint: OperatorInboxMirrorCheckpoint {
                peer_id: origin_node_id.to_string(),
                last_exported_sequence: checkpoint.current_sequence,
                last_acked_sequence: checkpoint.current_sequence,
                updated_at: checkpoint.updated_at,
            },
            checkpoint,
            applied_changes,
            skipped_changes,
        })
    }

    pub fn notification_candidates(
        &self,
        request: &OperatorNotificationListRequest,
    ) -> OrcasResult<OperatorNotificationListResponse> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| OrcasError::Store("mirror store connection lock poisoned".to_string()))?;
        let mut statement = connection.prepare(
            "select candidate_id, origin_node_id, item_id, trigger_sequence, candidate_status, item_json, created_at, updated_at, acknowledged_at, suppressed_at, resolved_at, obsolete_at
             from mirrored_notification_candidates
             where origin_node_id = ?1
             order by updated_at desc, candidate_id asc",
        ).map_err(db_error)?;
        let mut rows = statement
            .query(params![request.origin_node_id.as_str()])
            .map_err(db_error)?;
        let mut candidates = Vec::new();
        while let Some(row) = rows.next().map_err(db_error)? {
            let candidate = Self::candidate_from_row(row).map_err(db_error)?;
            if request.pending_only
                && candidate.status != OperatorNotificationCandidateStatus::Pending
            {
                continue;
            }
            if let Some(status) = request.status {
                if candidate.status != status {
                    continue;
                }
            }
            if request.actionable_only && !Self::candidate_is_actionable(&candidate) {
                continue;
            }
            candidates.push(candidate);
        }
        if let Some(limit) = request.limit {
            candidates.truncate(limit);
        }
        Ok(OperatorNotificationListResponse {
            origin_node_id: request.origin_node_id.clone(),
            candidates,
        })
    }

    pub fn notification_candidate(
        &self,
        request: &OperatorNotificationGetRequest,
    ) -> OrcasResult<Option<OperatorNotificationCandidate>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| OrcasError::Store("mirror store connection lock poisoned".to_string()))?;
        let mut statement = connection.prepare(
            "select candidate_id, origin_node_id, item_id, trigger_sequence, candidate_status, item_json, created_at, updated_at, acknowledged_at, suppressed_at, resolved_at, obsolete_at
             from mirrored_notification_candidates
             where origin_node_id = ?1 and candidate_id = ?2",
        ).map_err(db_error)?;
        let candidate = statement
            .query_row(
                params![
                    request.origin_node_id.as_str(),
                    request.candidate_id.as_str()
                ],
                |row| Self::candidate_from_row(row),
            )
            .optional()
            .map_err(db_error)?;
        Ok(candidate)
    }

    pub fn acknowledge_notification_candidate(
        &self,
        request: &OperatorNotificationAckRequest,
    ) -> OrcasResult<OperatorNotificationAckResponse> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| OrcasError::Store("mirror store connection lock poisoned".to_string()))?;
        let transaction = connection.transaction().map_err(db_error)?;
        let candidate = Self::load_notification_candidate_tx(
            &transaction,
            request.origin_node_id.as_str(),
            request.candidate_id.as_str(),
        )?
        .ok_or_else(|| OrcasError::Store("notification candidate not found".to_string()))?;
        let next = match candidate.status {
            OperatorNotificationCandidateStatus::Pending => {
                let updated_at = Utc::now();
                self.write_notification_candidate_status_tx(
                    &transaction,
                    request.origin_node_id.as_str(),
                    request.candidate_id.as_str(),
                    OperatorNotificationCandidateStatus::Acknowledged,
                    updated_at,
                    Some(updated_at),
                    candidate.suppressed_at,
                    candidate.resolved_at,
                    None,
                )?
            }
            OperatorNotificationCandidateStatus::Acknowledged => candidate,
            OperatorNotificationCandidateStatus::Suppressed => {
                return Err(OrcasError::Store(
                    "suppressed notification candidates cannot be acknowledged".to_string(),
                ));
            }
            OperatorNotificationCandidateStatus::Obsolete => {
                return Err(OrcasError::Store(
                    "obsolete notification candidates cannot be acknowledged".to_string(),
                ));
            }
        };
        transaction.commit().map_err(db_error)?;
        Ok(OperatorNotificationAckResponse { candidate: next })
    }

    pub fn suppress_notification_candidate(
        &self,
        request: &OperatorNotificationSuppressRequest,
    ) -> OrcasResult<OperatorNotificationSuppressResponse> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| OrcasError::Store("mirror store connection lock poisoned".to_string()))?;
        let transaction = connection.transaction().map_err(db_error)?;
        let candidate = Self::load_notification_candidate_tx(
            &transaction,
            request.origin_node_id.as_str(),
            request.candidate_id.as_str(),
        )?
        .ok_or_else(|| OrcasError::Store("notification candidate not found".to_string()))?;
        let next = match candidate.status {
            OperatorNotificationCandidateStatus::Pending
            | OperatorNotificationCandidateStatus::Acknowledged => {
                let updated_at = Utc::now();
                self.write_notification_candidate_status_tx(
                    &transaction,
                    request.origin_node_id.as_str(),
                    request.candidate_id.as_str(),
                    OperatorNotificationCandidateStatus::Suppressed,
                    updated_at,
                    candidate.acknowledged_at,
                    Some(updated_at),
                    candidate.resolved_at,
                    None,
                )?
            }
            OperatorNotificationCandidateStatus::Suppressed => candidate,
            OperatorNotificationCandidateStatus::Obsolete => {
                return Err(OrcasError::Store(
                    "obsolete notification candidates cannot be suppressed".to_string(),
                ));
            }
        };
        transaction.commit().map_err(db_error)?;
        Ok(OperatorNotificationSuppressResponse { candidate: next })
    }

    fn load_checkpoint_tx(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        origin_node_id: &str,
    ) -> OrcasResult<OperatorInboxCheckpoint> {
        let mut statement = transaction.prepare(
            "select current_sequence, updated_at from mirrored_inbox_checkpoint where origin_node_id = ?1",
        ).map_err(db_error)?;
        let checkpoint = statement
            .query_row(params![origin_node_id], |row| {
                Ok(OperatorInboxCheckpoint {
                    current_sequence: row.get::<_, i64>(0)? as u64,
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .map(|value| value.with_timezone(&Utc))
                        .map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                1,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?,
                })
            })
            .optional()
            .map_err(db_error)?;
        Ok(checkpoint.unwrap_or_default())
    }

    fn load_mirrored_item_tx(
        transaction: &rusqlite::Transaction<'_>,
        origin_node_id: &str,
        item_id: &str,
    ) -> OrcasResult<Option<OperatorInboxItem>> {
        let mut statement = transaction
            .prepare("select item_json from mirrored_inbox_items where origin_node_id = ?1 and item_id = ?2")
            .map_err(db_error)?;
        let item = statement
            .query_row(params![origin_node_id, item_id], |row| {
                let item_json = row.get::<_, String>(0)?;
                serde_json::from_str::<OperatorInboxItem>(&item_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })
            })
            .optional()
            .map_err(db_error)?;
        Ok(item)
    }

    fn candidate_from_row(
        row: &rusqlite::Row<'_>,
    ) -> Result<OperatorNotificationCandidate, rusqlite::Error> {
        let item_json = row.get::<_, String>(5)?;
        let item = serde_json::from_str::<OperatorInboxItem>(&item_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                5,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;
        let status = match row.get::<_, String>(4)?.as_str() {
            "pending" => OperatorNotificationCandidateStatus::Pending,
            "acknowledged" => OperatorNotificationCandidateStatus::Acknowledged,
            "suppressed" => OperatorNotificationCandidateStatus::Suppressed,
            "obsolete" => OperatorNotificationCandidateStatus::Obsolete,
            other => {
                return Err(rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("unknown notification candidate status `{other}`"),
                    )),
                ));
            }
        };
        let parse_ts = |index: usize| -> Result<DateTime<Utc>, rusqlite::Error> {
            DateTime::parse_from_rfc3339(&row.get::<_, String>(index)?)
                .map(|value| value.with_timezone(&Utc))
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        index,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })
        };
        let parse_optional_ts = |index: usize| -> Result<Option<DateTime<Utc>>, rusqlite::Error> {
            row.get::<_, Option<String>>(index)?
                .map(|value| {
                    DateTime::parse_from_rfc3339(&value)
                        .map(|value| value.with_timezone(&Utc))
                        .map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                index,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })
                })
                .transpose()
        };
        Ok(OperatorNotificationCandidate {
            candidate_id: row.get::<_, String>(0)?,
            origin_node_id: row.get::<_, String>(1)?,
            item_id: row.get::<_, String>(2)?,
            trigger_sequence: row.get::<_, i64>(3)? as u64,
            status,
            item,
            created_at: parse_ts(6)?,
            updated_at: parse_ts(7)?,
            acknowledged_at: parse_optional_ts(8)?,
            suppressed_at: parse_optional_ts(9)?,
            resolved_at: parse_optional_ts(10)?,
        })
    }

    fn inbox_item_is_actionable(item: &OperatorInboxItem) -> bool {
        item.status == orcas_core::ipc::OperatorInboxItemStatus::Open
            && !item.available_actions.is_empty()
    }

    fn candidate_is_actionable(candidate: &OperatorNotificationCandidate) -> bool {
        candidate.status != OperatorNotificationCandidateStatus::Obsolete
            && Self::inbox_item_is_actionable(&candidate.item)
    }

    fn notification_candidate_id(
        origin_node_id: &str,
        item_id: &str,
        trigger_sequence: u64,
    ) -> String {
        format!("{origin_node_id}::{item_id}::{trigger_sequence}")
    }

    fn candidate_status_to_str(status: OperatorNotificationCandidateStatus) -> &'static str {
        match status {
            OperatorNotificationCandidateStatus::Pending => "pending",
            OperatorNotificationCandidateStatus::Acknowledged => "acknowledged",
            OperatorNotificationCandidateStatus::Suppressed => "suppressed",
            OperatorNotificationCandidateStatus::Obsolete => "obsolete",
        }
    }

    fn load_notification_window_tx(
        transaction: &rusqlite::Transaction<'_>,
        origin_node_id: &str,
        item_id: &str,
    ) -> OrcasResult<Option<(String, u64)>> {
        let mut statement = transaction
            .prepare("select candidate_id, opened_sequence from mirrored_notification_windows where origin_node_id = ?1 and item_id = ?2")
            .map_err(db_error)?;
        let window = statement
            .query_row(params![origin_node_id, item_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
            })
            .optional()
            .map_err(db_error)?;
        Ok(window)
    }

    fn load_notification_candidate_tx(
        transaction: &rusqlite::Transaction<'_>,
        origin_node_id: &str,
        candidate_id: &str,
    ) -> OrcasResult<Option<OperatorNotificationCandidate>> {
        let mut statement = transaction
            .prepare("select candidate_id, origin_node_id, item_id, trigger_sequence, candidate_status, item_json, created_at, updated_at, acknowledged_at, suppressed_at, resolved_at, obsolete_at from mirrored_notification_candidates where origin_node_id = ?1 and candidate_id = ?2")
            .map_err(db_error)?;
        let candidate = statement
            .query_row(params![origin_node_id, candidate_id], |row| {
                Self::candidate_from_row(row)
            })
            .optional()
            .map_err(db_error)?;
        Ok(candidate)
    }

    fn apply_notification_transition_tx(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        origin_node_id: &str,
        previous_item: Option<&OperatorInboxItem>,
        current_item: Option<&OperatorInboxItem>,
        sequence: u64,
        changed_at: DateTime<Utc>,
    ) -> OrcasResult<()> {
        let previous_actionable = previous_item
            .map(Self::inbox_item_is_actionable)
            .unwrap_or(false);
        let current_actionable = current_item
            .map(Self::inbox_item_is_actionable)
            .unwrap_or(false);
        let item_id = current_item
            .map(|item| item.id.as_str())
            .or_else(|| previous_item.map(|item| item.id.as_str()))
            .ok_or_else(|| {
                OrcasError::Store("notification transition missing item identity".to_string())
            })?;
        let existing_window =
            Self::load_notification_window_tx(transaction, origin_node_id, item_id)?;

        match (previous_actionable, current_actionable) {
            (false, true) => {
                let item =
                    current_item.expect("current item should exist when becoming actionable");
                if existing_window.is_some() {
                    self.update_notification_candidate_snapshot_tx(
                        transaction,
                        origin_node_id,
                        item.id.as_str(),
                        item,
                        sequence,
                        changed_at,
                    )?;
                } else {
                    self.create_notification_candidate_tx(
                        transaction,
                        origin_node_id,
                        item,
                        sequence,
                        changed_at,
                    )?;
                }
            }
            (true, true) => {
                let item =
                    current_item.expect("current item should exist when remaining actionable");
                if existing_window.is_none() {
                    self.create_notification_candidate_tx(
                        transaction,
                        origin_node_id,
                        item,
                        sequence,
                        changed_at,
                    )?;
                } else {
                    self.update_notification_candidate_snapshot_tx(
                        transaction,
                        origin_node_id,
                        item.id.as_str(),
                        item,
                        sequence,
                        changed_at,
                    )?;
                }
            }
            (true, false) => {
                self.close_notification_candidate_tx(
                    transaction,
                    origin_node_id,
                    item_id,
                    current_item.or(previous_item),
                    changed_at,
                )?;
            }
            (false, false) => {}
        }
        Ok(())
    }

    fn create_notification_candidate_tx(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        origin_node_id: &str,
        item: &OperatorInboxItem,
        sequence: u64,
        changed_at: DateTime<Utc>,
    ) -> OrcasResult<()> {
        let candidate_id =
            Self::notification_candidate_id(origin_node_id, item.id.as_str(), sequence);
        let item_json = serde_json::to_string(item)?;
        transaction.execute(
            "insert into mirrored_notification_candidates(candidate_id, origin_node_id, item_id, trigger_sequence, candidate_status, item_json, created_at, updated_at, acknowledged_at, suppressed_at, resolved_at, obsolete_at)
             values(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, null, null, null, null)
             on conflict(candidate_id) do update set item_json = excluded.item_json, updated_at = excluded.updated_at",
            params![
                candidate_id,
                origin_node_id,
                item.id.as_str(),
                sequence as i64,
                Self::candidate_status_to_str(OperatorNotificationCandidateStatus::Pending),
                item_json,
                changed_at.to_rfc3339(),
                changed_at.to_rfc3339(),
            ],
        ).map_err(db_error)?;
        transaction.execute(
            "insert into mirrored_notification_windows(origin_node_id, item_id, candidate_id, opened_sequence, updated_sequence, updated_at)
             values(?1, ?2, ?3, ?4, ?5, ?6)
             on conflict(origin_node_id, item_id) do update set
               candidate_id = excluded.candidate_id,
               opened_sequence = excluded.opened_sequence,
               updated_sequence = excluded.updated_sequence,
               updated_at = excluded.updated_at",
            params![
                origin_node_id,
                item.id.as_str(),
                candidate_id,
                sequence as i64,
                sequence as i64,
                changed_at.to_rfc3339(),
            ],
        ).map_err(db_error)?;
        Ok(())
    }

    fn update_notification_candidate_snapshot_tx(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        origin_node_id: &str,
        item_id: &str,
        item: &OperatorInboxItem,
        sequence: u64,
        changed_at: DateTime<Utc>,
    ) -> OrcasResult<()> {
        let Some((candidate_id, _)) =
            Self::load_notification_window_tx(transaction, origin_node_id, item_id)?
        else {
            return self.create_notification_candidate_tx(
                transaction,
                origin_node_id,
                item,
                sequence,
                changed_at,
            );
        };
        let item_json = serde_json::to_string(item)?;
        transaction.execute(
            "update mirrored_notification_candidates set item_json = ?3, updated_at = ?4 where candidate_id = ?1 and origin_node_id = ?2",
            params![candidate_id, origin_node_id, item_json, changed_at.to_rfc3339()],
        ).map_err(db_error)?;
        transaction.execute(
            "update mirrored_notification_windows set updated_sequence = ?3, updated_at = ?4 where origin_node_id = ?1 and item_id = ?2",
            params![origin_node_id, item_id, sequence as i64, changed_at.to_rfc3339()],
        ).map_err(db_error)?;
        Ok(())
    }

    fn close_notification_candidate_tx(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        origin_node_id: &str,
        item_id: &str,
        item: Option<&OperatorInboxItem>,
        changed_at: DateTime<Utc>,
    ) -> OrcasResult<()> {
        let Some((candidate_id, _)) =
            Self::load_notification_window_tx(transaction, origin_node_id, item_id)?
        else {
            return Ok(());
        };
        if let Some(item_json) = item.map(serde_json::to_string).transpose()? {
            transaction
                .execute(
                    "update mirrored_notification_candidates
                 set item_json = ?3,
                     candidate_status = ?4,
                     updated_at = ?5,
                     resolved_at = coalesce(resolved_at, ?5),
                     obsolete_at = coalesce(obsolete_at, ?5)
                 where candidate_id = ?1 and origin_node_id = ?2",
                    params![
                        candidate_id,
                        origin_node_id,
                        item_json,
                        Self::candidate_status_to_str(
                            OperatorNotificationCandidateStatus::Obsolete
                        ),
                        changed_at.to_rfc3339(),
                    ],
                )
                .map_err(db_error)?;
        } else {
            transaction
                .execute(
                    "update mirrored_notification_candidates
                 set candidate_status = ?3,
                     updated_at = ?4,
                     resolved_at = coalesce(resolved_at, ?4),
                     obsolete_at = coalesce(obsolete_at, ?4)
                 where candidate_id = ?1 and origin_node_id = ?2",
                    params![
                        candidate_id,
                        origin_node_id,
                        Self::candidate_status_to_str(
                            OperatorNotificationCandidateStatus::Obsolete
                        ),
                        changed_at.to_rfc3339(),
                    ],
                )
                .map_err(db_error)?;
        }
        transaction.execute(
            "delete from mirrored_notification_windows where origin_node_id = ?1 and item_id = ?2",
            params![origin_node_id, item_id],
        ).map_err(db_error)?;
        Ok(())
    }

    fn write_notification_candidate_status_tx(
        &self,
        transaction: &rusqlite::Transaction<'_>,
        origin_node_id: &str,
        candidate_id: &str,
        status: OperatorNotificationCandidateStatus,
        updated_at: DateTime<Utc>,
        acknowledged_at: Option<DateTime<Utc>>,
        suppressed_at: Option<DateTime<Utc>>,
        resolved_at: Option<DateTime<Utc>>,
        obsolete_at: Option<DateTime<Utc>>,
    ) -> OrcasResult<OperatorNotificationCandidate> {
        let existing =
            Self::load_notification_candidate_tx(transaction, origin_node_id, candidate_id)?
                .ok_or_else(|| OrcasError::Store("notification candidate not found".to_string()))?;
        transaction
            .execute(
                "update mirrored_notification_candidates
             set candidate_status = ?3,
                 updated_at = ?4,
                 acknowledged_at = coalesce(?5, acknowledged_at),
                 suppressed_at = coalesce(?6, suppressed_at),
                 resolved_at = coalesce(?7, resolved_at),
                 obsolete_at = coalesce(?8, obsolete_at)
             where origin_node_id = ?1 and candidate_id = ?2",
                params![
                    origin_node_id,
                    candidate_id,
                    Self::candidate_status_to_str(status),
                    updated_at.to_rfc3339(),
                    acknowledged_at.map(|value| value.to_rfc3339()),
                    suppressed_at.map(|value| value.to_rfc3339()),
                    resolved_at.map(|value| value.to_rfc3339()),
                    obsolete_at.map(|value| value.to_rfc3339()),
                ],
            )
            .map_err(db_error)?;
        Self::load_notification_candidate_tx(transaction, origin_node_id, candidate_id)?
            .or(Some(existing))
            .ok_or_else(|| {
                OrcasError::Store("notification candidate disappeared during update".to_string())
            })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use tempfile::tempdir;

    use super::*;
    use orcas_core::ipc::{
        OperatorInboxActionKind, OperatorInboxChange, OperatorInboxChangeKind, OperatorInboxItem,
        OperatorInboxItemStatus, OperatorInboxSourceKind,
    };

    fn ts(offset: i64) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0)
            .single()
            .expect("base timestamp")
            + chrono::Duration::seconds(offset)
    }

    fn item(
        id: &str,
        sequence: u64,
        title: &str,
        updated_at: chrono::DateTime<Utc>,
    ) -> OperatorInboxItem {
        OperatorInboxItem {
            id: id.to_string(),
            sequence,
            source_kind: OperatorInboxSourceKind::SupervisorProposal,
            actionable_object_id: id.to_string(),
            workstream_id: Some("workstream-1".to_string()),
            work_unit_id: Some("work-unit-1".to_string()),
            title: title.to_string(),
            summary: format!("summary {title}"),
            status: OperatorInboxItemStatus::Open,
            available_actions: vec![OperatorInboxActionKind::Approve],
            created_at: updated_at,
            updated_at,
            resolved_at: None,
            rationale: None,
            provenance: Some("source=proposal".to_string()),
        }
    }

    fn change(
        sequence: u64,
        kind: OperatorInboxChangeKind,
        item: OperatorInboxItem,
    ) -> OperatorInboxChange {
        OperatorInboxChange {
            sequence,
            kind,
            item,
            changed_at: ts(sequence as i64),
        }
    }

    fn open_notification_request(origin_node_id: &str) -> OperatorNotificationListRequest {
        OperatorNotificationListRequest {
            origin_node_id: origin_node_id.to_string(),
            status: None,
            pending_only: false,
            actionable_only: false,
            limit: None,
        }
    }

    fn resolved_item(
        id: &str,
        sequence: u64,
        title: &str,
        updated_at: chrono::DateTime<Utc>,
    ) -> OperatorInboxItem {
        OperatorInboxItem {
            id: id.to_string(),
            sequence,
            source_kind: OperatorInboxSourceKind::SupervisorProposal,
            actionable_object_id: id.to_string(),
            workstream_id: Some("workstream-1".to_string()),
            work_unit_id: Some("work-unit-1".to_string()),
            title: title.to_string(),
            summary: format!("summary {title}"),
            status: OperatorInboxItemStatus::Resolved,
            available_actions: Vec::new(),
            created_at: updated_at,
            updated_at,
            resolved_at: Some(updated_at),
            rationale: None,
            provenance: Some("source=proposal".to_string()),
        }
    }

    #[test]
    fn apply_batch_is_idempotent_and_overlap_safe() {
        let dir = tempdir().expect("tempdir");
        let store = InboxMirrorStore::open(dir.path().join("server.db")).expect("store");
        let origin = "origin-a";

        let first = change(
            1,
            OperatorInboxChangeKind::Upsert,
            item("proposal-1", 1, "one", ts(1)),
        );
        let second = change(
            2,
            OperatorInboxChangeKind::Upsert,
            item("proposal-2", 2, "two", ts(2)),
        );
        let third = change(
            3,
            OperatorInboxChangeKind::Removed,
            item("proposal-1", 1, "one", ts(3)),
        );

        let result = store
            .apply_batch(
                origin,
                OperatorInboxCheckpoint::default(),
                &[first.clone(), second.clone()],
            )
            .expect("apply batch");
        assert_eq!(result.checkpoint.current_sequence, 2);
        assert_eq!(result.applied_changes, 2);

        let repeat = store
            .apply_batch(
                origin,
                result.checkpoint.clone(),
                &[first.clone(), second.clone()],
            )
            .expect("repeat batch");
        assert_eq!(repeat.checkpoint.current_sequence, 2);
        assert_eq!(repeat.applied_changes, 0);
        assert_eq!(store.list(origin, None).expect("list").items.len(), 2);

        let overlap = store
            .apply_batch(
                origin,
                result.checkpoint.clone(),
                &[second.clone(), third.clone()],
            )
            .expect("overlap batch");
        assert_eq!(overlap.checkpoint.current_sequence, 3);
        assert_eq!(overlap.applied_changes, 1);
        assert_eq!(store.list(origin, None).expect("list").items.len(), 1);
        assert_eq!(
            store.get(origin, "proposal-2").expect("get"),
            Some(item("proposal-2", 2, "two", ts(2)))
        );
    }

    #[test]
    fn checkpoint_persists_across_restart() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("server.db");
        let origin = "origin-a";
        let store = InboxMirrorStore::open(&path).expect("store");
        let change = change(
            1,
            OperatorInboxChangeKind::Upsert,
            item("proposal-1", 1, "one", ts(1)),
        );
        let result = store
            .apply_batch(origin, OperatorInboxCheckpoint::default(), &[change])
            .expect("apply");
        drop(store);

        let reopened = InboxMirrorStore::open(&path).expect("reopen");
        let checkpoint = reopened.checkpoint(origin).expect("checkpoint");
        assert_eq!(
            checkpoint.current_sequence,
            result.checkpoint.current_sequence
        );
        assert_eq!(reopened.list(origin, None).expect("list").items.len(), 1);
    }

    #[test]
    fn newly_actionable_item_creates_one_pending_notification_candidate() {
        let dir = tempdir().expect("tempdir");
        let store = InboxMirrorStore::open(dir.path().join("server.db")).expect("store");
        let origin = "origin-a";
        let action = change(
            1,
            OperatorInboxChangeKind::Upsert,
            item("proposal-1", 1, "one", ts(1)),
        );

        let result = store
            .apply_batch(origin, OperatorInboxCheckpoint::default(), &[action])
            .expect("apply");
        assert_eq!(result.checkpoint.current_sequence, 1);

        let candidates = store
            .notification_candidates(&open_notification_request(origin))
            .expect("candidates");
        assert_eq!(candidates.candidates.len(), 1);
        assert_eq!(
            candidates.candidates[0].status,
            OperatorNotificationCandidateStatus::Pending
        );
        assert_eq!(candidates.candidates[0].item.id, "proposal-1");
        assert_eq!(candidates.candidates[0].trigger_sequence, 1);
    }

    #[test]
    fn replayed_or_overlapping_batches_do_not_duplicate_candidates() {
        let dir = tempdir().expect("tempdir");
        let store = InboxMirrorStore::open(dir.path().join("server.db")).expect("store");
        let origin = "origin-a";
        let first = change(
            1,
            OperatorInboxChangeKind::Upsert,
            item("proposal-1", 1, "one", ts(1)),
        );
        let second = change(
            2,
            OperatorInboxChangeKind::Upsert,
            item("proposal-1", 2, "one-updated", ts(2)),
        );

        let result = store
            .apply_batch(
                origin,
                OperatorInboxCheckpoint::default(),
                &[first.clone(), second.clone()],
            )
            .expect("apply");
        let repeat = store
            .apply_batch(
                origin,
                result.checkpoint.clone(),
                &[first.clone(), second.clone()],
            )
            .expect("repeat");
        assert_eq!(repeat.applied_changes, 0);
        let overlap = store
            .apply_batch(origin, result.checkpoint.clone(), &[second.clone()])
            .expect("overlap");
        assert_eq!(overlap.applied_changes, 0);

        let candidates = store
            .notification_candidates(&open_notification_request(origin))
            .expect("candidates");
        assert_eq!(candidates.candidates.len(), 1);
        assert_eq!(
            candidates.candidates[0].candidate_id,
            format!("{origin}::proposal-1::1")
        );
        assert_eq!(candidates.candidates[0].item.title, "one-updated");
    }

    #[test]
    fn terminal_transition_obsoletes_notification_candidate() {
        let dir = tempdir().expect("tempdir");
        let store = InboxMirrorStore::open(dir.path().join("server.db")).expect("store");
        let origin = "origin-a";
        let open = change(
            1,
            OperatorInboxChangeKind::Upsert,
            item("proposal-1", 1, "one", ts(1)),
        );
        let closed = change(
            2,
            OperatorInboxChangeKind::Upsert,
            resolved_item("proposal-1", 2, "one", ts(2)),
        );

        store
            .apply_batch(origin, OperatorInboxCheckpoint::default(), &[open, closed])
            .expect("apply");

        let candidate = store
            .notification_candidate(&OperatorNotificationGetRequest {
                origin_node_id: origin.to_string(),
                candidate_id: format!("{origin}::proposal-1::1"),
            })
            .expect("candidate")
            .expect("candidate present");
        assert_eq!(
            candidate.status,
            OperatorNotificationCandidateStatus::Obsolete
        );
        assert!(candidate.resolved_at.is_some());
        assert!(candidate.item.available_actions.is_empty());
    }

    #[test]
    fn reopened_item_creates_a_new_candidate_window() {
        let dir = tempdir().expect("tempdir");
        let store = InboxMirrorStore::open(dir.path().join("server.db")).expect("store");
        let origin = "origin-a";
        let open = change(
            1,
            OperatorInboxChangeKind::Upsert,
            item("proposal-1", 1, "one", ts(1)),
        );
        let closed = change(
            2,
            OperatorInboxChangeKind::Upsert,
            resolved_item("proposal-1", 2, "one", ts(2)),
        );
        let reopen = change(
            3,
            OperatorInboxChangeKind::Upsert,
            item("proposal-1", 3, "reopened", ts(3)),
        );

        store
            .apply_batch(
                origin,
                OperatorInboxCheckpoint::default(),
                &[open, closed, reopen],
            )
            .expect("apply");

        let candidates = store
            .notification_candidates(&open_notification_request(origin))
            .expect("candidates");
        assert_eq!(candidates.candidates.len(), 2);
        assert!(
            candidates
                .candidates
                .iter()
                .any(
                    |candidate| candidate.candidate_id == format!("{origin}::proposal-1::1")
                        && candidate.status == OperatorNotificationCandidateStatus::Obsolete
                )
        );
        assert!(
            candidates
                .candidates
                .iter()
                .any(
                    |candidate| candidate.candidate_id == format!("{origin}::proposal-1::3")
                        && candidate.status == OperatorNotificationCandidateStatus::Pending
                )
        );
    }

    #[test]
    fn passive_or_closed_items_do_not_create_candidates() {
        let dir = tempdir().expect("tempdir");
        let store = InboxMirrorStore::open(dir.path().join("server.db")).expect("store");
        let origin = "origin-a";
        let passive = OperatorInboxItem {
            available_actions: Vec::new(),
            status: OperatorInboxItemStatus::Resolved,
            ..item("proposal-1", 1, "one", ts(1))
        };
        store
            .apply_batch(
                origin,
                OperatorInboxCheckpoint::default(),
                &[change(1, OperatorInboxChangeKind::Upsert, passive)],
            )
            .expect("apply");
        let candidates = store
            .notification_candidates(&open_notification_request(origin))
            .expect("candidates");
        assert!(candidates.candidates.is_empty());
    }

    #[test]
    fn candidate_ack_and_suppress_persist_across_restart() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("server.db");
        let origin = "origin-a";
        let store = InboxMirrorStore::open(&path).expect("store");
        store
            .apply_batch(
                origin,
                OperatorInboxCheckpoint::default(),
                &[change(
                    1,
                    OperatorInboxChangeKind::Upsert,
                    item("proposal-1", 1, "one", ts(1)),
                )],
            )
            .expect("apply");
        let candidate_id = format!("{origin}::proposal-1::1");
        let acked = store
            .acknowledge_notification_candidate(&OperatorNotificationAckRequest {
                origin_node_id: origin.to_string(),
                candidate_id: candidate_id.clone(),
            })
            .expect("ack");
        assert_eq!(
            acked.candidate.status,
            OperatorNotificationCandidateStatus::Acknowledged
        );
        let suppressed = store
            .suppress_notification_candidate(&OperatorNotificationSuppressRequest {
                origin_node_id: origin.to_string(),
                candidate_id: candidate_id.clone(),
            })
            .expect("suppress");
        assert_eq!(
            suppressed.candidate.status,
            OperatorNotificationCandidateStatus::Suppressed
        );
        drop(store);

        let reopened = InboxMirrorStore::open(&path).expect("reopen");
        let candidate = reopened
            .notification_candidate(&OperatorNotificationGetRequest {
                origin_node_id: origin.to_string(),
                candidate_id,
            })
            .expect("candidate")
            .expect("candidate present");
        assert_eq!(
            candidate.status,
            OperatorNotificationCandidateStatus::Suppressed
        );
        assert!(candidate.acknowledged_at.is_some());
        assert!(candidate.suppressed_at.is_some());
    }
}
