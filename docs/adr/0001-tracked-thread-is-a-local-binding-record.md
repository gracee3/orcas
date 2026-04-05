# ADR 0001: Tracked Thread Is A Local TT Record

## Status

Accepted

## Context

TT supervises work that may involve upstream TT runtime threads, but the TT daemon is supposed to become the local authority for operator state. The open design question is whether TT should persist upstream thread rows directly as mutable domain objects or whether it should persist its own tracked-thread record that may reference an upstream runtime thread.

The distinction matters for delete semantics, offline operation, and future sync.

## Decision

For the local-authority MVP, TT will persist an TT-owned `tracked_thread` record under a work unit.

That record may contain an `upstream_thread_id`, but it is not an upstream thread row. It is a local tracking and binding record.

`CreateTrackedThread` creates the local record.

`EditTrackedThread` changes local metadata and binding fields.

`DeleteTrackedThread` tombstones the local record only. It does not promise hard deletion of the upstream runtime thread.

## Consequences

Positive:

- TT can run fully offline and still own a complete local supervision model.
- Delete semantics stay honest.
- The operator client can treat tracked threads as normal local CRUD objects.
- Later sync can replicate TT records without pretending TT owns remote runtime storage.

Tradeoff:

- Some operators may initially expect a one-to-one identity between an TT tracked thread and a TT runtime thread. The product language needs to make the local-tracking semantics explicit.

## Follow-On Design

The local backend should therefore:

- keep tracked threads in SQLite as TT entities
- store optional upstream references
- use tombstones for delete
- expose tracked-thread CRUD over daemon IPC
- reserve sync metadata such as stable IDs, revisions, and origin node identity from day one
