#![allow(unused_crate_dependencies)]

mod harness;

use chrono::Utc;

use harness::TestDaemon;
use orcas_core::authority::{self, CommandActor, CommandMetadata, CorrelationId, OriginNodeId};
use orcas_core::{WorkUnitStatus, WorkstreamStatus, ipc};

struct AuthorityFixture {
    origin_node_id: OriginNodeId,
    actor: CommandActor,
}

impl AuthorityFixture {
    fn new() -> Self {
        Self {
            origin_node_id: OriginNodeId::new(),
            actor: CommandActor::parse("integration_test").expect("command actor"),
        }
    }

    fn metadata(&self, label: &str) -> CommandMetadata {
        CommandMetadata {
            command_id: authority::CommandId::new(),
            issued_at: Utc::now(),
            origin_node_id: self.origin_node_id.clone(),
            actor: self.actor.clone(),
            correlation_id: Some(
                CorrelationId::parse(format!("corr-{label}")).expect("correlation id"),
            ),
        }
    }
}

async fn create_authority_workstream(
    client: &orcasd::OrcasIpcClient,
    fixture: &AuthorityFixture,
    workstream_id: &str,
    title: &str,
) -> authority::WorkstreamRecord {
    client
        .authority_workstream_create(&ipc::AuthorityWorkstreamCreateRequest {
            command: authority::CreateWorkstream {
                metadata: fixture.metadata("bridge-ws-create"),
                workstream_id: authority::WorkstreamId::parse(workstream_id)
                    .expect("workstream id"),
                title: title.to_string(),
                objective: format!("Objective for {title}"),
                status: WorkstreamStatus::Active,
                priority: "high".to_string(),
            },
        })
        .await
        .expect("create authority workstream")
        .workstream
}

async fn create_authority_workunit(
    client: &orcasd::OrcasIpcClient,
    fixture: &AuthorityFixture,
    work_unit_id: &str,
    workstream_id: &authority::WorkstreamId,
    title: &str,
) -> authority::WorkUnitRecord {
    client
        .authority_workunit_create(&ipc::AuthorityWorkunitCreateRequest {
            command: authority::CreateWorkUnit {
                metadata: fixture.metadata("bridge-wu-create"),
                work_unit_id: authority::WorkUnitId::parse(work_unit_id).expect("work unit id"),
                workstream_id: workstream_id.clone(),
                title: title.to_string(),
                task_statement: format!("Task for {title}"),
                status: WorkUnitStatus::Ready,
            },
        })
        .await
        .expect("create authority work unit")
        .work_unit
}

#[tokio::test]
async fn authority_hierarchy_projects_into_collaboration_state_and_events() {
    let mut daemon = TestDaemon::spawn("authority-bridge").await;
    let client = daemon.connect().await;
    let fixture = AuthorityFixture::new();

    let (mut events, snapshot) = client
        .subscribe_events(true)
        .await
        .expect("subscribe to daemon events");
    let snapshot = snapshot.expect("snapshot should be returned");
    assert!(snapshot.collaboration.workstreams.is_empty());
    assert!(snapshot.collaboration.work_units.is_empty());

    let workstream = create_authority_workstream(
        &client,
        &fixture,
        "authority-bridge-ws",
        "Bridged Workstream",
    )
    .await;
    let work_unit = create_authority_workunit(
        &client,
        &fixture,
        "authority-bridge-wu",
        &workstream.id,
        "Bridged Work Unit",
    )
    .await;

    let workstream_event = TestDaemon::next_event_matching(&mut events, |envelope| {
        matches!(
            &envelope.event,
            ipc::DaemonEvent::WorkstreamLifecycle { action, workstream: summary }
                if *action == ipc::CollaborationLifecycleAction::Created
                    && summary.id == workstream.id.as_str()
        )
    })
    .await;
    match workstream_event.event {
        ipc::DaemonEvent::WorkstreamLifecycle { action, workstream } => {
            assert_eq!(action, ipc::CollaborationLifecycleAction::Created);
            assert_eq!(workstream.id, workstream.id);
            assert_eq!(workstream.title, "Bridged Workstream");
        }
        other => panic!("expected workstream lifecycle event, got {other:?}"),
    }

    let work_unit_event = TestDaemon::next_event_matching(&mut events, |envelope| {
        matches!(
            &envelope.event,
            ipc::DaemonEvent::WorkUnitLifecycle { action, work_unit: summary }
                if *action == ipc::CollaborationLifecycleAction::Created
                    && summary.id == work_unit.id.as_str()
        )
    })
    .await;
    match work_unit_event.event {
        ipc::DaemonEvent::WorkUnitLifecycle { action, work_unit } => {
            assert_eq!(action, ipc::CollaborationLifecycleAction::Created);
            assert_eq!(work_unit.id, work_unit.id);
            assert_eq!(work_unit.workstream_id, workstream.id.as_str());
            assert_eq!(work_unit.title, "Bridged Work Unit");
        }
        other => panic!("expected work unit lifecycle event, got {other:?}"),
    }

    let projected = client
        .state_get()
        .await
        .expect("state/get after authority projection");
    let projected_workstream = projected
        .snapshot
        .collaboration
        .workstreams
        .iter()
        .find(|summary| summary.id == workstream.id.as_str())
        .expect("authority workstream should project into collaboration snapshot");
    assert_eq!(projected_workstream.title, "Bridged Workstream");
    assert_eq!(projected_workstream.status, WorkstreamStatus::Active);

    let projected_work_unit = projected
        .snapshot
        .collaboration
        .work_units
        .iter()
        .find(|summary| summary.id == work_unit.id.as_str())
        .expect("authority work unit should project into collaboration snapshot");
    assert_eq!(projected_work_unit.workstream_id, workstream.id.as_str());
    assert_eq!(projected_work_unit.title, "Bridged Work Unit");
    assert_eq!(projected_work_unit.status, WorkUnitStatus::Ready);

    daemon.stop().await;
}

#[tokio::test]
async fn projected_authority_hierarchy_persists_into_state_after_restart() {
    let mut daemon = TestDaemon::spawn("authority-bridge-restart").await;
    let fixture = AuthorityFixture::new();

    let first_client = daemon.connect().await;
    let workstream = create_authority_workstream(
        &first_client,
        &fixture,
        "authority-bridge-ws-restart",
        "Restart Bridge Workstream",
    )
    .await;
    let work_unit = create_authority_workunit(
        &first_client,
        &fixture,
        "authority-bridge-wu-restart",
        &workstream.id,
        "Restart Bridge Work Unit",
    )
    .await;

    daemon.restart().await;

    let second_client = daemon.connect().await;
    let projected = second_client
        .state_get()
        .await
        .expect("state/get after restart");

    assert!(
        projected
            .snapshot
            .collaboration
            .workstreams
            .iter()
            .any(|summary| summary.id == workstream.id.as_str()
                && summary.title == "Restart Bridge Workstream")
    );
    assert!(
        projected
            .snapshot
            .collaboration
            .work_units
            .iter()
            .any(|summary| summary.id == work_unit.id.as_str()
                && summary.workstream_id == workstream.id.as_str()
                && summary.title == "Restart Bridge Work Unit")
    );

    daemon.stop().await;
}
