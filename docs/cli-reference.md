# TT CLI Reference

_Generated from the current `tt` Clap command tree._

## `tt`

TT v2 local client

**Usage**

```text
Usage: tt [OPTIONS] <COMMAND>
```

**Subcommands**

- `doctor`
- `status`
- `repo`
- `project`
- `codex`
- `workspace`
- `records`
- `docs`

**Arguments**

- `--cwd` `<CWD>` (optional): Working directory to open the TT runtime in

### `tt doctor`

**Usage**

```text
Usage: doctor [OPTIONS]
```

**Arguments**

- `--codex` (optional)
- `--check-listen` (optional)

### `tt status`

**Usage**

```text
Usage: status
```

### `tt repo`

**Usage**

```text
Usage: repo
```

### `tt project`

**Usage**

```text
Usage: project <COMMAND>
```

**Subcommands**

- `inspect`
- `plan`
- `init`
- `open`
- `director`
- `control`
- `spawn`
- `attach`

#### `tt project inspect`

**Usage**

```text
Usage: inspect
```

#### `tt project plan`

**Usage**

```text
Usage: plan <COMMAND>
```

**Subcommands**

- `show`
- `refresh`

##### `tt project plan show`

**Usage**

```text
Usage: show
```

##### `tt project plan refresh`

**Usage**

```text
Usage: refresh
```

#### `tt project init`

**Usage**

```text
Usage: init [OPTIONS]
```

**Arguments**

- `--path` `<PATH>` (optional)
- `--title` `<TITLE>` (optional)
- `--objective` `<OBJECTIVE>` (optional)
- `--template` `<TEMPLATE>` (optional)
- `--base-branch` `<BASE_BRANCH>` (optional)
- `--worktree-root` `<WORKTREE_ROOT>` (optional)
- `--director-model` `<DIRECTOR_MODEL>` (optional)
- `--dev-model` `<DEV_MODEL>` (optional)
- `--test-model` `<TEST_MODEL>` (optional)
- `--integration-model` `<INTEGRATION_MODEL>` (optional)

#### `tt project open`

**Usage**

```text
Usage: open [OPTIONS]
```

**Arguments**

- `--title` `<TITLE>` (optional)
- `--objective` `<OBJECTIVE>` (optional)
- `--base-branch` `<BASE_BRANCH>` (optional)
- `--worktree-root` `<WORKTREE_ROOT>` (optional)
- `--director-model` `<DIRECTOR_MODEL>` (optional)
- `--dev-model` `<DEV_MODEL>` (optional)
- `--test-model` `<TEST_MODEL>` (optional)
- `--integration-model` `<INTEGRATION_MODEL>` (optional)

#### `tt project director`

**Usage**

```text
Usage: director [OPTIONS]
```

**Arguments**

- `--title` `<TITLE>` (optional)
- `--objective` `<OBJECTIVE>` (optional)
- `--base-branch` `<BASE_BRANCH>` (optional)
- `--worktree-root` `<WORKTREE_ROOT>` (optional)
- `--director-model` `<DIRECTOR_MODEL>` (optional)
- `--dev-model` `<DEV_MODEL>` (optional)
- `--test-model` `<TEST_MODEL>` (optional)
- `--integration-model` `<INTEGRATION_MODEL>` (optional)
- `--role` `<ROLE>` (optional)
- `--binding` `<BINDING>` (optional)
- `--scenario` `<SCENARIO>` (optional)
- `--seed-file` `<SEED_FILE>` (optional)

#### `tt project control`

**Usage**

```text
Usage: control --role <ROLE> --mode <MODE>
```

**Arguments**

- `--role` `<ROLE>`
- `--mode` `<MODE>`

#### `tt project spawn`

**Usage**

```text
Usage: spawn [OPTIONS]
```

**Arguments**

- `--role` `<ROLE>` (optional)

#### `tt project attach`

**Usage**

```text
Usage: attach [OPTIONS]
```

**Arguments**

- `--binding` `<BINDING>` (optional)

### `tt codex`

**Usage**

```text
Usage: codex <COMMAND>
```

**Subcommands**

- `threads`
- `app-servers`

#### `tt codex threads`

**Usage**

```text
Usage: threads <COMMAND>
```

**Subcommands**

- `list`
- `get`
- `read`
- `start`
- `resume`

##### `tt codex threads list`

**Usage**

```text
Usage: list [LIMIT]
```

**Arguments**

- `limit` `<LIMIT>` (optional)

##### `tt codex threads get`

**Usage**

```text
Usage: get <SELECTOR>
```

**Arguments**

- `selector` `<SELECTOR>`

##### `tt codex threads read`

**Usage**

```text
Usage: read [OPTIONS] <SELECTOR>
```

**Arguments**

- `selector` `<SELECTOR>`
- `--include-turns` (optional)

##### `tt codex threads start`

**Usage**

```text
Usage: start [OPTIONS]
```

**Arguments**

- `--model` `<MODEL>` (optional)
- `--ephemeral` (optional)

##### `tt codex threads resume`

**Usage**

```text
Usage: resume [OPTIONS] <SELECTOR>
```

**Arguments**

- `selector` `<SELECTOR>`
- `--model` `<MODEL>` (optional)

#### `tt codex app-servers`

**Usage**

```text
Usage: app-servers
```

### `tt workspace`

**Usage**

```text
Usage: workspace <COMMAND>
```

**Subcommands**

- `binding`
- `merge-run`
- `action`
- `lifecycle`

#### `tt workspace binding`

**Usage**

```text
Usage: binding <COMMAND>
```

**Subcommands**

- `list`
- `get`
- `upsert`
- `set-status`
- `refresh`
- `delete`

##### `tt workspace binding list`

**Usage**

```text
Usage: list
```

##### `tt workspace binding get`

**Usage**

```text
Usage: get <ID>
```

**Arguments**

- `id` `<ID>`

##### `tt workspace binding upsert`

**Usage**

```text
Usage: upsert <FILE>
```

**Arguments**

- `file` `<FILE>`

##### `tt workspace binding set-status`

**Usage**

```text
Usage: set-status <ID> <STATUS>
```

**Arguments**

- `id` `<ID>`
- `status` `<STATUS>`

##### `tt workspace binding refresh`

**Usage**

```text
Usage: refresh <ID>
```

**Arguments**

- `id` `<ID>`

##### `tt workspace binding delete`

**Usage**

```text
Usage: delete <ID>
```

**Arguments**

- `id` `<ID>`

#### `tt workspace merge-run`

**Usage**

```text
Usage: merge-run <COMMAND>
```

**Subcommands**

- `list`
- `get`
- `upsert`
- `set-status`
- `refresh`
- `delete`

##### `tt workspace merge-run list`

**Usage**

```text
Usage: list
```

##### `tt workspace merge-run get`

**Usage**

```text
Usage: get <ID>
```

**Arguments**

- `id` `<ID>`

##### `tt workspace merge-run upsert`

**Usage**

```text
Usage: upsert <FILE>
```

**Arguments**

- `file` `<FILE>`

##### `tt workspace merge-run set-status`

**Usage**

```text
Usage: set-status <ID> <READINESS> <AUTHORIZATION> <EXECUTION> [HEAD_COMMIT]
```

**Arguments**

- `id` `<ID>`
- `readiness` `<READINESS>`
- `authorization` `<AUTHORIZATION>`
- `execution` `<EXECUTION>`
- `head_commit` `<HEAD_COMMIT>` (optional)

##### `tt workspace merge-run refresh`

**Usage**

```text
Usage: refresh <WORKSPACE_BINDING_ID>
```

**Arguments**

- `workspace_binding_id` `<WORKSPACE_BINDING_ID>`

##### `tt workspace merge-run delete`

**Usage**

```text
Usage: delete <ID>
```

**Arguments**

- `id` `<ID>`

#### `tt workspace action`

**Usage**

```text
Usage: action <COMMAND>
```

**Subcommands**

- `prepare`
- `refresh`
- `merge-prep`
- `authorize-merge`
- `execute-landing`
- `prune`

##### `tt workspace action prepare`

**Usage**

```text
Usage: prepare <ID>
```

**Arguments**

- `id` `<ID>`

##### `tt workspace action refresh`

**Usage**

```text
Usage: refresh <ID>
```

**Arguments**

- `id` `<ID>`

##### `tt workspace action merge-prep`

**Usage**

```text
Usage: merge-prep <ID>
```

**Arguments**

- `id` `<ID>`

##### `tt workspace action authorize-merge`

**Usage**

```text
Usage: authorize-merge <ID>
```

**Arguments**

- `id` `<ID>`

##### `tt workspace action execute-landing`

**Usage**

```text
Usage: execute-landing <ID>
```

**Arguments**

- `id` `<ID>`

##### `tt workspace action prune`

**Usage**

```text
Usage: prune [OPTIONS] <ID>
```

**Arguments**

- `id` `<ID>`
- `--force` (optional)

#### `tt workspace lifecycle`

**Usage**

```text
Usage: lifecycle <COMMAND>
```

**Subcommands**

- `close`
- `park`
- `split`

##### `tt workspace lifecycle close`

**Usage**

```text
Usage: close [OPTIONS] [SELECTOR]
```

**Arguments**

- `selector` `<SELECTOR>` (optional)
- `--force` (optional)

##### `tt workspace lifecycle park`

**Usage**

```text
Usage: park [OPTIONS] [SELECTOR]
```

**Arguments**

- `selector` `<SELECTOR>` (optional)
- `--note` `<NOTE>` (optional)

##### `tt workspace lifecycle split`

**Usage**

```text
Usage: split [OPTIONS]
```

**Arguments**

- `--role` `<ROLE>` (optional)
- `--model` `<MODEL>` (optional)
- `--ephemeral` (optional)

### `tt records`

**Usage**

```text
Usage: records <COMMAND>
```

**Subcommands**

- `project`
- `work-unit`
- `thread-binding`

#### `tt records project`

**Usage**

```text
Usage: project <COMMAND>
```

**Subcommands**

- `list`
- `get`
- `upsert`
- `set-status`
- `delete`

##### `tt records project list`

**Usage**

```text
Usage: list
```

##### `tt records project get`

**Usage**

```text
Usage: get <ID_OR_SLUG>
```

**Arguments**

- `id_or_slug` `<ID_OR_SLUG>`

##### `tt records project upsert`

**Usage**

```text
Usage: upsert <FILE>
```

**Arguments**

- `file` `<FILE>`

##### `tt records project set-status`

**Usage**

```text
Usage: set-status <ID_OR_SLUG> <STATUS>
```

**Arguments**

- `id_or_slug` `<ID_OR_SLUG>`
- `status` `<STATUS>`

##### `tt records project delete`

**Usage**

```text
Usage: delete <ID_OR_SLUG>
```

**Arguments**

- `id_or_slug` `<ID_OR_SLUG>`

#### `tt records work-unit`

**Usage**

```text
Usage: work-unit <COMMAND>
```

**Subcommands**

- `list`
- `get`
- `upsert`
- `set-status`
- `delete`

##### `tt records work-unit list`

**Usage**

```text
Usage: list [PROJECT_ID]
```

**Arguments**

- `project_id` `<PROJECT_ID>` (optional)

##### `tt records work-unit get`

**Usage**

```text
Usage: get <ID_OR_SLUG>
```

**Arguments**

- `id_or_slug` `<ID_OR_SLUG>`

##### `tt records work-unit upsert`

**Usage**

```text
Usage: upsert <FILE>
```

**Arguments**

- `file` `<FILE>`

##### `tt records work-unit set-status`

**Usage**

```text
Usage: set-status <ID_OR_SLUG> <STATUS>
```

**Arguments**

- `id_or_slug` `<ID_OR_SLUG>`
- `status` `<STATUS>`

##### `tt records work-unit delete`

**Usage**

```text
Usage: delete <ID_OR_SLUG>
```

**Arguments**

- `id_or_slug` `<ID_OR_SLUG>`

#### `tt records thread-binding`

**Usage**

```text
Usage: thread-binding <COMMAND>
```

**Subcommands**

- `list`
- `get`
- `upsert`
- `set-status`
- `delete`

##### `tt records thread-binding list`

**Usage**

```text
Usage: list
```

##### `tt records thread-binding get`

**Usage**

```text
Usage: get <CODEX_THREAD_ID>
```

**Arguments**

- `codex_thread_id` `<CODEX_THREAD_ID>`

##### `tt records thread-binding upsert`

**Usage**

```text
Usage: upsert <FILE>
```

**Arguments**

- `file` `<FILE>`

##### `tt records thread-binding set-status`

**Usage**

```text
Usage: set-status <CODEX_THREAD_ID> <STATUS>
```

**Arguments**

- `codex_thread_id` `<CODEX_THREAD_ID>`
- `status` `<STATUS>`

##### `tt records thread-binding delete`

**Usage**

```text
Usage: delete <CODEX_THREAD_ID>
```

**Arguments**

- `codex_thread_id` `<CODEX_THREAD_ID>`

### `tt docs`

**Usage**

```text
Usage: docs <COMMAND>
```

**Subcommands**

- `export-cli-ref`

#### `tt docs export-cli-ref`

**Usage**

```text
Usage: export-cli-ref [OPTIONS]
```

**Arguments**

- `--output` `<OUTPUT>` (optional)

