use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};

use orcas_codex::types;
use orcas_codex::{
    CodexClient, CodexDaemonManager, DaemonLaunch, LocalCodexDaemonManager, WebSocketTransport,
};
use orcas_core::{
    AppConfig, AppPaths, CodexConnectionMode, JsonSessionStore, OrcasEvent, OrcasSessionStore,
    ThreadMetadata,
};

#[derive(Debug, Clone, Default)]
pub struct RuntimeOverrides {
    pub codex_bin: Option<PathBuf>,
    pub listen_url: Option<String>,
    pub cwd: Option<PathBuf>,
    pub model: Option<String>,
    pub connect_only: bool,
    pub force_spawn: bool,
}

pub struct SupervisorService {
    pub paths: AppPaths,
    pub config: AppConfig,
    pub store: JsonSessionStore,
    daemon: LocalCodexDaemonManager,
    client: Arc<CodexClient>,
}

impl SupervisorService {
    pub async fn load(overrides: &RuntimeOverrides) -> Result<Self> {
        let paths = AppPaths::discover()?;
        paths.ensure().await?;
        let mut config = AppConfig::write_default_if_missing(&paths).await?;
        if let Some(codex_bin) = &overrides.codex_bin {
            config.codex.binary_path = codex_bin.clone();
        }
        if let Some(listen_url) = &overrides.listen_url {
            config.codex.listen_url = listen_url.clone();
        }
        if let Some(cwd) = &overrides.cwd {
            config.defaults.cwd = Some(cwd.clone());
        }
        if let Some(model) = &overrides.model {
            config.defaults.model = Some(model.clone());
        }
        if overrides.connect_only {
            config.codex.connection_mode = CodexConnectionMode::ConnectOnly;
        }
        let store = JsonSessionStore::new(paths.clone(), config.clone());
        let daemon =
            LocalCodexDaemonManager::new(config.codex.clone(), &paths, config.defaults.cwd.clone());
        let client = CodexClient::new(
            Arc::new(WebSocketTransport::new(config.codex.listen_url.clone())),
            config.codex.reconnect.clone(),
            Arc::new(orcas_codex::RejectingApprovalRouter),
        );

        Ok(Self {
            paths,
            config,
            store,
            daemon,
            client,
        })
    }

    pub async fn doctor(&self) -> Result<()> {
        let status = self.daemon.status().await?;
        println!("config: {}", self.paths.config_file.display());
        println!("state: {}", self.paths.state_file.display());
        println!("codex_bin: {}", status.binary_path.display());
        println!("endpoint: {}", status.endpoint);
        println!("reachable: {}", status.reachable);
        println!("log_file: {}", status.log_path.display());
        Ok(())
    }

    pub async fn daemon_status(&self) -> Result<()> {
        let status = self.daemon.status().await?;
        println!("endpoint: {}", status.endpoint);
        println!("reachable: {}", status.reachable);
        println!("binary: {}", status.binary_path.display());
        println!("log_file: {}", status.log_path.display());
        Ok(())
    }

    pub async fn daemon_start(&self, force: bool) -> Result<()> {
        let launch = if force {
            DaemonLaunch::Always
        } else {
            DaemonLaunch::IfNeeded
        };
        let status = self.daemon.ensure_running(launch).await?;
        println!("endpoint: {}", status.endpoint);
        println!("reachable: {}", status.reachable);
        println!("log_file: {}", status.log_path.display());
        Ok(())
    }

    pub async fn models_list(&self) -> Result<()> {
        self.ensure_ready().await?;
        let response = self
            .client
            .model_list(types::ModelListParams::default())
            .await?;
        for model in response.data {
            println!(
                "{}\t{}\thidden={}\tdefault={}",
                model.model, model.display_name, model.hidden, model.is_default
            );
        }
        Ok(())
    }

    pub async fn threads_list(&self) -> Result<()> {
        self.ensure_ready().await?;
        let response = self
            .client
            .thread_list(types::ThreadListParams::default())
            .await?;
        self.persist_threads(&response.data, None).await?;
        for thread in response.data {
            println!(
                "{}\t{}\t{}\t{}",
                thread.id,
                thread.status.label(),
                thread.model_provider,
                thread.preview.replace('\n', " ")
            );
        }
        Ok(())
    }

    pub async fn thread_read(&self, thread_id: &str) -> Result<()> {
        self.ensure_ready().await?;
        let response = self
            .client
            .thread_read(types::ThreadReadParams {
                thread_id: thread_id.to_string(),
                include_turns: true,
            })
            .await?;
        self.persist_thread(&response.thread, None).await?;
        println!("thread: {}", response.thread.id);
        println!("status: {}", response.thread.status.label());
        println!("cwd: {}", response.thread.cwd);
        println!("preview: {}", response.thread.preview);
        println!("turns: {}", response.thread.turns.len());
        Ok(())
    }

    pub async fn thread_start(
        &self,
        cwd: Option<PathBuf>,
        model: Option<String>,
        ephemeral: bool,
    ) -> Result<String> {
        self.ensure_ready().await?;
        let params = types::ThreadStartParams {
            cwd: cwd
                .or_else(|| self.config.defaults.cwd.clone())
                .map(|path| path.display().to_string()),
            model: model.or_else(|| self.config.defaults.model.clone()),
            ephemeral: Some(ephemeral),
            ..types::ThreadStartParams::default()
        };
        let response = self.client.thread_start(params).await?;
        self.persist_thread(&response.thread, Some(response.model))
            .await?;
        println!("thread_id: {}", response.thread.id);
        Ok(response.thread.id)
    }

    pub async fn thread_resume(
        &self,
        thread_id: &str,
        cwd: Option<PathBuf>,
        model: Option<String>,
    ) -> Result<String> {
        self.ensure_ready().await?;
        let response = self
            .client
            .thread_resume(types::ThreadResumeParams {
                thread_id: thread_id.to_string(),
                cwd: cwd
                    .or_else(|| self.config.defaults.cwd.clone())
                    .map(|path| path.display().to_string()),
                model: model.or_else(|| self.config.defaults.model.clone()),
                approval_policy: Some(types::AskForApproval::default()),
                approvals_reviewer: None,
                sandbox: None,
                config: None,
                base_instructions: None,
                developer_instructions: None,
                persist_extended_history: true,
            })
            .await?;
        self.persist_thread(&response.thread, Some(response.model))
            .await?;
        println!("thread_id: {}", response.thread.id);
        Ok(response.thread.id)
    }

    pub async fn prompt(&self, thread_id: &str, text: &str) -> Result<String> {
        self.ensure_ready().await?;
        self.send_turn(thread_id, text, true).await
    }

    pub async fn quickstart(
        &self,
        cwd: Option<PathBuf>,
        model: Option<String>,
        text: &str,
    ) -> Result<()> {
        let thread_id = self.thread_start(cwd, model, false).await?;
        let final_text = self.send_turn(&thread_id, text, false).await?;
        println!("\nthread_id: {thread_id}");
        println!("final_text_len: {}", final_text.len());
        Ok(())
    }

    async fn send_turn(&self, thread_id: &str, text: &str, resume_thread: bool) -> Result<String> {
        if resume_thread {
            let resumed = self
                .client
                .thread_resume(types::ThreadResumeParams {
                    thread_id: thread_id.to_string(),
                    cwd: self
                        .config
                        .defaults
                        .cwd
                        .clone()
                        .map(|path| path.display().to_string()),
                    model: self.config.defaults.model.clone(),
                    approval_policy: Some(types::AskForApproval::default()),
                    approvals_reviewer: None,
                    sandbox: None,
                    config: None,
                    base_instructions: None,
                    developer_instructions: None,
                    persist_extended_history: true,
                })
                .await?;
            self.persist_thread(&resumed.thread, Some(resumed.model))
                .await?;
        }

        let mut events = self.client.subscribe();
        let response = self
            .client
            .turn_start(types::TurnStartParams {
                thread_id: thread_id.to_string(),
                input: vec![types::UserInput::Text {
                    text: text.to_string(),
                    text_elements: Vec::new(),
                }],
                cwd: None,
                approval_policy: Some(types::AskForApproval::default()),
                approvals_reviewer: None,
                sandbox_policy: None,
                model: None,
            })
            .await?;
        self.stream_turn(thread_id, &response.turn.id, &mut events)
            .await
    }

    async fn ensure_ready(&self) -> Result<()> {
        let launch = if self.config.codex.connection_mode == CodexConnectionMode::ConnectOnly {
            DaemonLaunch::Never
        } else {
            DaemonLaunch::IfNeeded
        };
        self.daemon.ensure_running(launch).await?;
        self.client.connect().await?;
        let _ = self
            .client
            .initialize(types::InitializeParams {
                client_info: types::ClientInfo {
                    name: "orcas-supervisor".to_string(),
                    title: Some("Orcas Supervisor".to_string()),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
                capabilities: Some(types::InitializeCapabilities {
                    experimental_api: true,
                    opt_out_notification_methods: None,
                }),
            })
            .await?;
        Ok(())
    }

    async fn stream_turn(
        &self,
        thread_id: &str,
        turn_id: &str,
        events: &mut tokio::sync::broadcast::Receiver<orcas_core::EventEnvelope>,
    ) -> Result<String> {
        let mut buffer = String::new();
        loop {
            match events.recv().await {
                Ok(envelope) => match envelope.event {
                    OrcasEvent::AgentMessageDelta {
                        thread_id: event_thread_id,
                        turn_id: event_turn_id,
                        delta,
                        ..
                    } if event_thread_id == thread_id && event_turn_id == turn_id => {
                        print!("{delta}");
                        io::stdout().flush().ok();
                        buffer.push_str(&delta);
                    }
                    OrcasEvent::TurnCompleted {
                        thread_id: event_thread_id,
                        turn_id: event_turn_id,
                        status,
                    } if event_thread_id == thread_id && event_turn_id == turn_id => {
                        println!("\n[turn completed: {status}]");
                        return Ok(buffer);
                    }
                    OrcasEvent::Warning { message } => {
                        eprintln!("warning: {message}");
                    }
                    OrcasEvent::ServerRequest { method } => {
                        eprintln!("server request pending: {method}");
                    }
                    _ => {}
                },
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    eprintln!("warning: event stream lagged, skipped {skipped} events");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    anyhow::bail!("event stream closed before turn completed")
                }
            }
        }
    }

    async fn persist_threads(
        &self,
        threads: &[types::Thread],
        model: Option<String>,
    ) -> Result<()> {
        for thread in threads {
            self.persist_thread(thread, model.clone()).await?;
        }
        Ok(())
    }

    async fn persist_thread(&self, thread: &types::Thread, model: Option<String>) -> Result<()> {
        let created_at = Utc
            .timestamp_opt(thread.created_at, 0)
            .single()
            .unwrap_or_else(Utc::now);
        let updated_at = Utc
            .timestamp_opt(thread.updated_at, 0)
            .single()
            .unwrap_or(created_at);
        self.store
            .upsert_thread(ThreadMetadata {
                id: thread.id.clone(),
                name: thread.name.clone(),
                preview: thread.preview.clone(),
                model,
                model_provider: Some(thread.model_provider.clone()),
                cwd: if thread.cwd.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(&thread.cwd))
                },
                endpoint: Some(self.config.codex.listen_url.clone()),
                created_at,
                updated_at,
                status: thread.status.label().to_string(),
            })
            .await
            .context("persist thread metadata")?;
        Ok(())
    }
}
