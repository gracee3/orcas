create table if not exists projects (
  id text primary key,
  slug text not null unique,
  title text not null,
  objective text not null,
  status text not null,
  created_at text not null,
  updated_at text not null
);

create table if not exists work_units (
  id text primary key,
  project_id text not null references projects(id) on delete cascade,
  slug text,
  title text not null,
  task text not null,
  status text not null,
  created_at text not null,
  updated_at text not null
);

create unique index if not exists work_units_project_slug_idx
  on work_units(project_id, slug)
  where slug is not null;

create table if not exists work_unit_dependencies (
  work_unit_id text not null references work_units(id) on delete cascade,
  depends_on_work_unit_id text not null references work_units(id) on delete cascade,
  primary key (work_unit_id, depends_on_work_unit_id)
);

create table if not exists thread_bindings (
  codex_thread_id text primary key,
  work_unit_id text references work_units(id) on delete set null,
  role text not null,
  status text not null,
  notes text,
  created_at text not null,
  updated_at text not null
);

create index if not exists thread_bindings_work_unit_idx
  on thread_bindings(work_unit_id, updated_at desc);

create table if not exists workspace_bindings (
  id text primary key,
  codex_thread_id text not null references thread_bindings(codex_thread_id) on delete cascade,
  repo_root text not null,
  worktree_path text,
  branch_name text,
  base_ref text,
  base_commit text,
  landing_target text,
  strategy text not null,
  sync_policy text not null,
  cleanup_policy text not null,
  status text not null,
  created_at text not null,
  updated_at text not null
);

create index if not exists workspace_bindings_thread_idx
  on workspace_bindings(codex_thread_id, updated_at desc);

create table if not exists merge_runs (
  id text primary key,
  workspace_binding_id text not null references workspace_bindings(id) on delete cascade,
  readiness text not null,
  authorization text not null,
  execution text not null,
  head_commit text,
  created_at text not null,
  updated_at text not null
);

create table if not exists review_runs (
  id text primary key,
  work_unit_id text not null references work_units(id) on delete cascade,
  codex_thread_id text references thread_bindings(codex_thread_id) on delete set null,
  kind text not null,
  status text not null,
  summary text,
  created_at text not null,
  updated_at text not null
);

create table if not exists todo_ledgers (
  id text primary key,
  project_id text references projects(id) on delete cascade,
  work_unit_id text references work_units(id) on delete cascade,
  codex_thread_id text references thread_bindings(codex_thread_id) on delete set null,
  status text not null,
  created_at text not null,
  updated_at text not null
);

create table if not exists reconcile_checkpoints (
  key text primary key,
  value_json text not null,
  updated_at text not null
);
