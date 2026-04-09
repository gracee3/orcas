# Managed Project Phased Director

This scenario boots a fresh managed project and activates the director/dev/test team in phases.

It verifies:

- `tt project open` creates the managed-project scaffold only.
- `tt project director --role director --role dev --role test` advances the project to a partial attachment state.
- `tt project director --role integration` completes the role topology.
- `tt project inspect` and `tt project status` report the current state consistently.
- TT records the resulting thread and workspace bindings for all four roles.
