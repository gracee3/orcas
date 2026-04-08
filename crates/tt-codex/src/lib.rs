//! Codex integration layer for TT v2.
//!
//! This crate owns Codex home discovery and lightweight catalog access for
//! TT. It does not reimplement Codex runtime behavior.

use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tt_domain as _;

pub const CODEX_HOME_ENV: &str = "CODEX_HOME";
pub const CODEX_SQLITE_HOME_ENV: &str = "CODEX_SQLITE_HOME";
pub const SESSION_INDEX_FILE: &str = "session_index.jsonl";
pub const CODEX_STATE_DB_FILENAME: &str = "state_5.sqlite";
pub const CODEX_LOGS_DB_FILENAME: &str = "logs_1.sqlite";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexHome {
    root: PathBuf,
}

impl CodexHome {
    pub fn discover() -> Result<Self> {
        Self::discover_from(env::var_os(CODEX_HOME_ENV), dirs::home_dir())
    }

    pub fn discover_in(cwd: impl AsRef<Path>) -> Result<Self> {
        let codex_dir = cwd.as_ref().join(".codex");
        if codex_dir.is_dir() {
            return Ok(Self::from_path(codex_dir));
        }
        Self::discover()
    }

    pub fn from_path(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        self.root.as_path()
    }

    pub fn state_db_path(&self) -> PathBuf {
        self.root.join(CODEX_STATE_DB_FILENAME)
    }

    pub fn logs_db_path(&self) -> PathBuf {
        self.root.join(CODEX_LOGS_DB_FILENAME)
    }

    pub fn session_index_path(&self) -> PathBuf {
        self.root.join(SESSION_INDEX_FILE)
    }

    pub fn session_catalog(&self) -> Result<CodexSessionCatalog> {
        CodexSessionCatalog::load(self.root())
    }

    fn discover_from(env_value: Option<OsString>, home_dir: Option<PathBuf>) -> Result<Self> {
        if let Some(value) = env_value {
            let root = PathBuf::from(value);
            if root.is_dir() {
                return Ok(Self { root });
            }
            anyhow::bail!("{} is set but is not a directory", CODEX_HOME_ENV);
        }

        let Some(home) = home_dir else {
            anyhow::bail!("could not resolve a home directory for Codex");
        };
        Ok(Self {
            root: home.join(".codex"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionIndexEntry {
    pub id: String,
    pub thread_name: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexThreadRecord {
    pub thread_id: String,
    pub thread_name: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexSessionCatalog {
    pub codex_home: CodexHome,
    pub threads: Vec<CodexThreadRecord>,
}

impl CodexSessionCatalog {
    pub fn load(root: &Path) -> Result<Self> {
        let codex_home = CodexHome::from_path(root);
        let path = codex_home.session_index_path();
        let mut threads = Vec::new();

        if path.exists() {
            let file = File::open(&path)
                .with_context(|| format!("open Codex session index {}", path.display()))?;
            for line in BufReader::new(file).lines() {
                let line = line?;
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(entry) = serde_json::from_str::<SessionIndexEntry>(trimmed) else {
                    continue;
                };
                threads.push(CodexThreadRecord {
                    thread_id: entry.id,
                    thread_name: (!entry.thread_name.trim().is_empty())
                        .then_some(entry.thread_name),
                    updated_at: DateTime::parse_from_rfc3339(&entry.updated_at)
                        .map(|value| value.with_timezone(&Utc))
                        .ok(),
                });
            }
        }

        threads.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| right.thread_id.cmp(&left.thread_id))
        });

        Ok(Self {
            codex_home,
            threads,
        })
    }

    pub fn find_thread_by_id(&self, thread_id: &str) -> Option<&CodexThreadRecord> {
        self.threads
            .iter()
            .find(|record| record.thread_id == thread_id)
    }

    pub fn find_thread_by_name(&self, thread_name: &str) -> Option<&CodexThreadRecord> {
        self.threads
            .iter()
            .find(|record| record.thread_name.as_deref() == Some(thread_name))
    }

    pub fn resolve_thread(&self, selector: &str) -> Option<&CodexThreadRecord> {
        self.find_thread_by_id(selector)
            .or_else(|| self.find_thread_by_name(selector))
            .or_else(|| {
                self.threads.iter().find(|record| {
                    record
                        .thread_id
                        .split_once(':')
                        .is_some_and(|(_, suffix)| suffix == selector)
                })
            })
    }

    pub fn recent_threads(&self, limit: usize) -> Vec<CodexThreadRecord> {
        self.threads.iter().take(limit).cloned().collect()
    }

    pub fn all_threads(&self) -> &[CodexThreadRecord] {
        &self.threads
    }
}

pub fn discover_codex_home() -> Result<CodexHome> {
    CodexHome::discover()
}

pub fn codex_state_db_path(codex_home: &Path) -> PathBuf {
    codex_home.join(CODEX_STATE_DB_FILENAME)
}

pub fn codex_logs_db_path(codex_home: &Path) -> PathBuf {
    codex_home.join(CODEX_LOGS_DB_FILENAME)
}

pub fn codex_session_index_path(codex_home: &Path) -> PathBuf {
    codex_home.join(SESSION_INDEX_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn discover_uses_environment_override() {
        let dir = tempdir().expect("tempdir");
        let discovered = CodexHome::discover_from(
            Some(dir.path().as_os_str().to_os_string()),
            Some(PathBuf::from("/tmp/fallback")),
        )
        .expect("discover codex home");

        assert_eq!(discovered.root(), dir.path());
    }

    #[test]
    fn catalog_loads_session_index() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join(SESSION_INDEX_FILE);
        std::fs::write(
            &path,
            concat!(
                "{\"id\":\"a\",\"thread_name\":\"alpha\",\"updated_at\":\"2026-04-08T12:00:00Z\"}\n",
                "{\"id\":\"b\",\"thread_name\":\"\",\"updated_at\":\"2026-04-08T12:01:00Z\"}\n"
            ),
        )
        .expect("write session index");

        let catalog = CodexSessionCatalog::load(dir.path()).expect("load catalog");

        assert_eq!(catalog.threads.len(), 2);
        assert_eq!(
            catalog
                .find_thread_by_id("a")
                .and_then(|record| record.thread_name.as_deref()),
            Some("alpha")
        );
        assert!(catalog.find_thread_by_name("alpha").is_some());
    }
}
