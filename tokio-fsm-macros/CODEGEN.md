# tokio-fsm-macros Codegen Notes

This crate follows a simple pipeline:

1. Parse macro input into `validation::FsmStructure`
2. Validate graph shape and handler semantics
3. Generate enums, structs, and impls

## Where to edit things

### Parse and validation

- `src/validation/parser.rs`
  Use for attribute parsing, handler return-shape parsing, and discovered states/events.
- `src/validation/graph.rs`
  Use for graph-level correctness checks such as reachability and declared
  transition targets.

### Type generation

- `src/codegen/enums.rs`
  Generated state and event enums.
- `src/codegen/structs.rs`
  Generated FSM, handle, and task structs.

### Runtime code generation

- `src/codegen/impls/spawn.rs`
  Spawn entry points and token ownership.
- `src/codegen/impls/run.rs`
  Event loop, cancellation, replies to spawned `apply`, and tracing instrumentation.
- `src/codegen/impls/handle.rs`
  Public spawned-runtime handle methods such as `apply` and `wait_for_state`.
- `src/codegen/impls/task.rs`
  Task future wrapper and drop behavior.
- `src/codegen/impls/helpers.rs`
  Small TokenStream builders plus generated private FSM helpers such as
  transition application.

## Working rules

- Keep parsed semantic branching in codegen.
  Example: `HandlerReturnKind` should stay a codegen decision.
- Move repetitive generated runtime tails into one generated private helper.
  Example: state update, watch notification, and tracing.
- Prefer small TokenStream builders over one giant helper with too many cases.
- Split files by responsibility before adding abstraction layers.
- Avoid traits or larger IR layers unless a new requirement clearly demands them.

## Current design intent

The macro should read like a straightforward code generator, not a framework.

The code should make it easy to answer:
- where does this behavior come from?
- is this decided by parsing, validation, or codegen?
- which generated surface changes if I edit this file?
