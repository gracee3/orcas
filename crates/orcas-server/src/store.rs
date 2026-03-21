use std::path::Path;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use orcas_core::ipc::{
    OperatorInboxCheckpoint, OperatorInboxChange, OperatorInboxItem, OperatorInboxMirrorCheckpoint,
    OperatorInboxMirrorListResponse,
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

    pub fn get(&self, origin_node_id: &str, item_id: &str) -> OrcasResult<Option<OperatorInboxItem>> {
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
                }
                orcas_core::ipc::OperatorInboxChangeKind::Removed => {
                    transaction.execute(
                        "delete from mirrored_inbox_items where origin_node_id = ?1 and item_id = ?2",
                        params![origin_node_id, change.item.id.as_str()],
                    ).map_err(db_error)?;
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

    fn item(id: &str, sequence: u64, title: &str, updated_at: chrono::DateTime<Utc>) -> OperatorInboxItem {
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

    fn change(sequence: u64, kind: OperatorInboxChangeKind, item: OperatorInboxItem) -> OperatorInboxChange {
        OperatorInboxChange {
            sequence,
            kind,
            item,
            changed_at: ts(sequence as i64),
        }
    }

    #[test]
    fn apply_batch_is_idempotent_and_overlap_safe() {
        let dir = tempdir().expect("tempdir");
        let store = InboxMirrorStore::open(dir.path().join("server.db")).expect("store");
        let origin = "origin-a";

        let first = change(1, OperatorInboxChangeKind::Upsert, item("proposal-1", 1, "one", ts(1)));
        let second = change(2, OperatorInboxChangeKind::Upsert, item("proposal-2", 2, "two", ts(2)));
        let third = change(3, OperatorInboxChangeKind::Removed, item("proposal-1", 1, "one", ts(3)));

        let result = store
            .apply_batch(origin, OperatorInboxCheckpoint::default(), &[first.clone(), second.clone()])
            .expect("apply batch");
        assert_eq!(result.checkpoint.current_sequence, 2);
        assert_eq!(result.applied_changes, 2);

        let repeat = store
            .apply_batch(origin, result.checkpoint.clone(), &[first.clone(), second.clone()])
            .expect("repeat batch");
        assert_eq!(repeat.checkpoint.current_sequence, 2);
        assert_eq!(repeat.applied_changes, 0);
        assert_eq!(store.list(origin, None).expect("list").items.len(), 2);

        let overlap = store
            .apply_batch(origin, result.checkpoint.clone(), &[second.clone(), third.clone()])
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
        let change = change(1, OperatorInboxChangeKind::Upsert, item("proposal-1", 1, "one", ts(1)));
        let result = store
            .apply_batch(origin, OperatorInboxCheckpoint::default(), &[change])
            .expect("apply");
        drop(store);

        let reopened = InboxMirrorStore::open(&path).expect("reopen");
        let checkpoint = reopened.checkpoint(origin).expect("checkpoint");
        assert_eq!(checkpoint.current_sequence, result.checkpoint.current_sequence);
        assert_eq!(reopened.list(origin, None).expect("list").items.len(), 1);
    }
}
