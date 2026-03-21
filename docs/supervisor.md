# Orcas Communication Architecture: Agent ↔ Supervisor ↔ Operator

## Purpose

This document explains the **code-level flow and logic** of Orcas’s three-layer communication system:

- **Agent / worker**
- **Supervisor**
- **Operator / human reviewer**

This is **not** a runtime-UX document. It intentionally focuses on:

- what the code constructs
- what gets persisted
- what gets validated
- how decisions turn into follow-on work

The key architectural idea is that Orcas is **not** just “sending prompts around.” It is trying to turn language interactions into **typed, versioned, inspectable protocol/state objects**.

---

## The big picture

Orcas has three distinct layers:

1. **Agent / worker layer**
   - Turns an assignment seed plus state into a concrete worker prompt.
   - Stores the rendered prompt and its structured packet.
   - Parses and validates the worker’s returned report.

2. **Supervisor layer**
   - Builds a structured context pack for supervisory reasoning.
   - Sends fixed instructions plus serialized state to a reasoning backend.
   - Persists the supervisor-side prompt artifact, response artifact, and parsed proposal.

3. **Operator / human layer**
   - Does not operate through “another prompt template.”
   - Operates through a durable proposal-review state machine.
   - Approves, rejects, or lets proposals go stale/superseded.
   - Approval can synthesize the next assignment.

The three layers are related, but they are **not symmetric**.

---

## Architectural thesis

The most important design choice in Orcas is:

> **Worker and supervisor interactions are not treated as ephemeral strings.**
> They are progressively modeled as persisted, inspectable artifacts tied to domain state.

That matters because it lets Orcas answer:

- What exactly did we ask the worker to do?
- What exact protocol/version governed that interaction?
- What did the supervisor actually see?
- What did the model return?
- What did the human approve?
- What assignment was created next, and from what lineage?

---

# 1. Core mental model

## 1.1 Worker layer = execution protocol

The worker layer is the most protocolized and artifact-oriented part of the system.

Conceptually:

- Orcas takes structured assignment state.
- It builds a **versioned packet**.
- It renders a **concrete worker prompt**.
- It persists both the packet and the render artifact.
- It sends the exact rendered text to the worker.
- It parses the returned report against the original packet and contract.

This is the clearest example of Orcas’s value proposition: the “prompt” is really a **rendered protocol boundary** backed by structured data.

## 1.2 Supervisor layer = decision protocol

The supervisor layer is similar in spirit, but historically was less artifactized.

Conceptually:

- Orcas builds a **SupervisorContextPack** from state.
- Orcas combines that with **fixed supervisory instructions**.
- Orcas sends the request to a reasoning backend under a strict schema.
- Orcas parses and validates the returned proposal.
- Orcas persists a proposal record that becomes the review object.

Later hardening made this more artifact-oriented by persisting:

- a **SupervisorPromptRenderArtifact**
- a **SupervisorResponseArtifact**

So the supervisor path is now much closer to the worker path in auditability, even though the shapes differ.

## 1.3 Operator layer = review state machine

The operator layer is not modeled as another prompt exchange.

Instead, Orcas treats human review as a **proposal lifecycle** over durable state:

- Open
- Approved
- Rejected
- Superseded
- Stale
- GenerationFailed

Approval is what bridges supervisory reasoning back into execution.

---

# 2. Main domain objects

## 2.1 Worker-side objects

Important worker-side types include:

- `AssignmentCommunicationSeed`
- `AssignmentCommunicationPacket`
- `PromptRenderSpec`
- `PromptRenderArtifact`
- `AssignmentCommunicationRecord`

These live primarily under:

- `crates/orcas-core/src/communication.rs`
- `crates/orcasd/src/assignment_comm/render.rs`
- `crates/orcasd/src/assignment_comm/policy.rs`
- `crates/orcasd/src/assignment_comm/parse.rs`

### What they mean

#### `AssignmentCommunicationSeed`
The structured source material for a worker assignment.

It carries things like:

- objective
- instructions
- acceptance criteria
- stop conditions
- context references
- provenance fields
- boundedness / mode information

It is the structured “why/what” of the assignment before prompt rendering.

#### `AssignmentCommunicationPacket`
The concrete packet Orcas derives from the seed plus current assignment/work-unit/workstream context.

This is the **worker-facing protocol object**.

It includes:

- identity
- provenance
- objective
- instructions
- scope boundaries
- non-goals
- included context
- response contract
- policy

#### `PromptRenderSpec`
The declarative metadata about how the worker prompt is rendered.

It controls things like:

- template version
- section order
- response markers
- style

It does **not** itself hold the prompt text.

#### `PromptRenderArtifact`
The actual rendered worker prompt artifact.

It includes:

- rendered prompt text
- render spec
- hashes / timing metadata

#### `AssignmentCommunicationRecord`
The canonical persisted worker communication record.

It ties together:

- packet
- prompt render artifact
- hashes
- parsed response envelope
- validation result
- raw output linkage

This is the authoritative worker interaction record.

---

## 2.2 Supervisor-side objects

Important supervisor-side types include:

- `SupervisorContextPack`
- `SupervisorProposal`
- `SupervisorProposalRecord`
- `SupervisorPromptRenderArtifact`
- `SupervisorResponseArtifact`

These live primarily under:

- `crates/orcas-core/src/supervisor.rs`
- `crates/orcasd/src/supervisor.rs`
- `crates/orcasd/src/service.rs`

### What they mean

#### `SupervisorContextPack`
The structured state packet the supervisor reasoner sees.

It includes things like:

- trigger metadata
- state anchor / freshness guardrails
- decision policy
- workstream context
- primary work-unit context
- source report context
- current assignment context
- worker session context
- dependency / related work context
- optional operator request

This is the supervisor-side equivalent of “what the model is allowed to reason over.”

#### `SupervisorProposal`
The structured output object Orcas expects back from the reasoner.

It is schema-constrained and then validated again by Orcas policy.

#### `SupervisorProposalRecord`
The durable review object.

This is the canonical human-review record. It persists:

- context pack
- reasoner metadata
- parsed proposal
- prompt artifact
- response artifact
- human edits
- approval/rejection metadata
- failure metadata
- final status

#### `SupervisorPromptRenderArtifact`
The persisted representation of what Orcas semantically sent to the supervisor reasoner.

It typically includes:

- supervisor prompt template version
- instruction text
- user content text
- serialized context pack text
- prompt hash
- optional request-body hash
- rendered timestamp

#### `SupervisorResponseArtifact`
The persisted representation of what came back from the supervisor reasoner.

It typically includes:

- backend/model/response id
- usage
- normalized output items
- extracted output text
- response hash
- optional raw response body
- optional raw response-body hash
- captured timestamp

This makes the supervisor path much more auditable than a plain “parsed JSON result only” model.

---

## 2.3 Operator-side objects

The operator layer mostly centers on:

- `SupervisorProposalRecord`
- proposal lifecycle status
- approval/rejection transitions
- optional human edits
- decision application

The important conceptual point is:

> The operator is reviewing a **proposal record**, not a prompt transcript.

That is why Orcas’s human layer feels more like workflow/state management than prompting.

---

# 3. What the worker prompt actually is

## 3.1 The worker prompt is rendered in code

The worker prompt is not a separate free-floating template file.

It is rendered in Rust in:

- `crates/orcasd/src/assignment_comm/render.rs`

The relevant flow is approximately:

1. Build packet from structured seed/state.
2. Validate packet.
3. Render prompt text from the packet.
4. Persist the packet and render artifact in `AssignmentCommunicationRecord`.
5. Send the exact persisted prompt text to the worker.

## 3.2 The worker prompt is versioned

The worker path has explicit version constants, including concepts like:

- packet version
- prompt template version
- worker report contract version
- worker report envelope version
- response markers

These live around:

- `crates/orcasd/src/assignment_comm/mod.rs`

This is important because it means Orcas is not treating the prompt as “just some text.” It is treating it as part of a versioned protocol contract.

## 3.3 Worker prompt section logic

The worker prompt has an explicit section order declared in code.

The important logical sections are roughly:

- worker contract
- assignment identity
- task mode
- objective
- instructions
- scope and non-goals
- acceptance criteria
- stop conditions
- included context
- response contract
- response example

These are rendered in `render_prompt(...)`.

## 3.4 What the worker is expected to return

The worker is not free to return arbitrary prose.

It is expected to return a **single envelope** between fixed markers, containing the report in the required shape.

Orcas then:

- extracts the envelope
- decodes the JSON
- validates it against the original packet and report contract
- records parse/validation status

This is the second half of the worker protocol: not just “what we asked,” but “what counts as a valid answer.”

---

# 4. What the supervisor prompt actually is

## 4.1 The supervisor prompt is also code, not a template file

The supervisor request is built in code in:

- `crates/orcasd/src/supervisor.rs`

Conceptually it consists of:

1. **Fixed supervisory instructions**
2. **Pretty-serialized `SupervisorContextPack`**
3. **Strict output schema requirements**

So the supervisor model is not improvising freely. It is being asked to produce a very specific structured proposal over a bounded state pack.

## 4.2 The supervisor request shape

The supervisor request body is conceptually:

- system/instruction text telling the model:
  - Orcas state is authoritative
  - choose only allowed decisions
  - do not invent IDs or hidden context
  - output strict JSON only

plus

- user content containing something like:
  - “Return a supervisor proposal JSON object...”
  - then the serialized `SupervisorContextPack`

plus

- strict JSON-schema response formatting

## 4.3 The supervisor prompt is now persisted as an artifact

Originally, the exact supervisor boundary was more reconstructable than inspectable.

That gap has now been closed architecturally by persisting a first-class supervisor prompt render artifact.

That means Orcas now retains:

- what instructions were used
- what serialized context pack text was sent
- what version/hash identified the render

This was one of the biggest observability wins in the hardening work.

## 4.4 The supervisor response is now also persisted as an artifact

Likewise, Orcas now preserves a first-class response-side artifact that records what came back from the reasoner.

That makes the supervisor flow much easier to inspect and audit:

- prompt artifact = what was sent
- response artifact = what came back
- parsed proposal = what Orcas accepted from it

This separation is important.

---

# 5. The operator layer is not “a third prompt”

This is the most important conceptual clarification.

## 5.1 What the operator is really doing

The operator is not sending another prompt in the same pattern.

Instead, the operator is reviewing and transitioning a **proposal record**.

That review can include:

- approval
- rejection
- human edits
- leaving stale proposals alone
- triggering supersession behavior

## 5.2 Why this matters

If the human layer were modeled as “just another prompt,” the system would lose a lot of workflow clarity.

By modeling it as state transitions over durable proposal records, Orcas gets:

- explicit lifecycle states
- explicit review metadata
- edit history / approval metadata
- structured linkage into next assignments

That is why the operator layer is best understood as **workflow/state control**, not language-generation control.

---

# 6. End-to-end flow

This is the main flow from assignment to next assignment.

## 6.1 Assignment seed → worker packet

A structured `AssignmentCommunicationSeed` is obtained from:

- manual assignment creation
- prior decisions/proposals
- follow-on assignment generation
- legacy/fallback instruction paths when necessary

That seed is turned into an `AssignmentCommunicationPacket`.

## 6.2 Worker packet → rendered worker prompt

Orcas renders the worker prompt from the packet and stores:

- packet
- render spec
- rendered prompt text
- hashes

inside `AssignmentCommunicationRecord`.

## 6.3 Rendered prompt → worker execution

When execution starts, Orcas sends the exact persisted prompt text to the worker.

This matters:
the authoritative prompt is the persisted render artifact, not an ad hoc regenerated string.

## 6.4 Worker output → parsed report

When the worker turn ends, Orcas:

- reads the raw output
- extracts the report envelope
- parses it
- validates it against the original packet
- records trust/validation outcomes
- stores the raw output and parsed report linkage

## 6.5 Parsed report → supervisor proposal generation

A report can trigger supervisor reasoning.

Orcas then:

- builds `SupervisorContextPack`
- renders supervisor prompt artifact
- sends request to the reasoning backend
- captures response artifact
- extracts output text
- parses `SupervisorProposal`
- validates it
- persists `SupervisorProposalRecord`

## 6.6 Proposal record → operator review

The operator reviews the persisted proposal record.

Possible states/transitions include:

- Open
- Approved
- Rejected
- Superseded
- Stale
- GenerationFailed

Freshness checks can mark proposals stale if state has moved on.

## 6.7 Approval → next assignment

When a proposal is approved:

1. Orcas applies human edits if present.
2. Orcas revalidates the resulting proposal.
3. Orcas compiles follow-on assignment instructions/preview text.
4. Orcas derives a structured next `AssignmentCommunicationSeed`.
5. Orcas creates the next assignment.
6. Orcas immediately ensures the next worker communication record exists.

The key point is:

> The approved supervisor proposal does not itself become the next raw prompt.
> It becomes structured assignment state, which is then re-rendered into the worker protocol.

That separation is one of the cleaner architectural choices in the system.

---

# 7. Why this design is valuable

## 7.1 It turns prompts into protocol artifacts

Instead of thinking:

- “LLM prompt in, LLM output out”

Orcas thinks more like:

- structured state
- protocol packet
- rendered artifact
- validated response
- persisted audit trail

That is a much stronger architecture for anything that needs accountability.

## 7.2 It preserves lineage

The system can trace:

- source report
- source proposal
- predecessor assignment
- approved decision
- next assignment

So the system is not merely generating text; it is building a lineage of execution and decision-making.

## 7.3 It separates reasoning from control

The model can propose.

Orcas policy and operator review decide what is valid and what gets applied.

This is a strong boundary:

- model = reasoning assistant
- Orcas = protocol/state authority
- operator = final reviewer where required

## 7.4 It makes debugging possible

Because artifacts are persisted, Orcas can answer questions like:

- why was this report trusted or downgraded?
- what exact worker prompt was sent?
- what exact supervisor state pack was shown?
- what did the model return?
- why did proposal generation fail?
- what did the human approve?

That is much harder in systems that only keep a thin prompt/result shell.

---

# 8. What is canonical vs convenience

This is a useful distinction.

## 8.1 Canonical execution/review truth

Canonical truth lives in persisted protocol/state records such as:

- `AssignmentCommunicationRecord`
- `Report`
- `SupervisorProposalRecord`
- `Assignment`
- decision/application records

These are the core architectural objects.

## 8.2 Convenience/inspection surfaces

Later work added explicit inspection/export surfaces such as:

- bounded artifact summary IPC
- full artifact detail IPC
- workunit-scoped summary list IPC
- CLI inspection/export
- operator client inspection/export

These are useful, but they are **not the architecture itself**.

They are observability surfaces over the canonical records.

That distinction is important if you want to avoid getting lost in the weeds.

---

# 9. The simplest way to think about the three roles

## Agent / worker
“I execute bounded work under a strict report contract.”

The worker is governed by:

- the rendered assignment prompt
- bounded scope
- acceptance criteria
- stop conditions
- required report envelope

## Supervisor
“I reason over structured state and propose the next decision in a constrained schema.”

The supervisor is governed by:

- fixed instructions
- serialized state pack
- decision policy
- schema-constrained proposal output
- Orcas-side validation

## Operator
“I review durable proposals and decide what actually changes state.”

The operator is governed by:

- proposal lifecycle state
- freshness constraints
- approval/rejection semantics
- optional edits
- application of approved decisions into next assignments

---

# 10. Short code map

## Worker path

- `crates/orcas-core/src/communication.rs`
  - worker-side communication types
- `crates/orcasd/src/assignment_comm/mod.rs`
  - version constants / markers
- `crates/orcasd/src/assignment_comm/render.rs`
  - packet building and prompt rendering
- `crates/orcasd/src/assignment_comm/policy.rs`
  - packet / envelope validation
- `crates/orcasd/src/assignment_comm/parse.rs`
  - parse worker report from raw output
- `crates/orcasd/src/service.rs`
  - execution start, report ingestion, persistence

## Supervisor path

- `crates/orcas-core/src/supervisor.rs`
  - context pack, proposal record, prompt/response artifacts
- `crates/orcasd/src/supervisor.rs`
  - request rendering, response extraction, proposal validation
- `crates/orcasd/src/service.rs`
  - proposal generation, stale/supersede handling, approval/rejection, decision application

## Operator / review path

- `crates/orcasd/src/service.rs`
  - proposal lifecycle transitions
- operator-client/CLI layers
  - inspection/export only; not the core logic

---

# 11. Bottom line

The heart of Orcas’s value is not “it has prompts.”

It is that Orcas tries to make **agent execution, supervisory reasoning, and human review legible as structured protocol/state transitions**.

The cleanest summary is:

- **Worker:** rendered protocol for execution
- **Supervisor:** structured protocol for decision proposal
- **Operator:** durable workflow for approval and state change

That is the real architecture.