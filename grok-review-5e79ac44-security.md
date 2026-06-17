# Security Review — Optic narrow v0 (iteration 5e79ac44)

**Reviewer:** Security engineer (focused audit)  
**Scope:** Input validation, run harness / command execution, CLI path handling, diagnostics/logging, temp-file handling  
**Baseline:** Implementer summary at `/tmp/grok-impl-summary-5e79ac44.md`

---

## Executive Summary

This iteration fixes a real **denial-of-service** bug in the parser recovery loop (infinite stall on malformed fn bodies / typed lets). The run harness was implemented with sound basics: `Command::new` argument vector (no shell), and `tempfile::tempdir()` instead of a predictable shared directory.

No **critical/high exploitable vulnerability** (command injection, codegen escape to arbitrary Rust, or cross-user temp-dir hijack) was found in the current code. Remaining findings are defense-in-depth and deployment-context items appropriate for a developer compiler CLI that intentionally compiles and executes generated code via `optic run`.

---

## Findings

### 1. Run harness executes generated native code with full process privileges

| Field | Value |
|---|---|
| **Severity** | suggestion |
| **File:Line** | `crates/optic-cli/src/main.rs:142-146`, `358-407` |
| **Description** | `optic run` (and `optic bench`) transpile user-supplied `.opt` input, write `main.rs` into a temp crate, and invoke `cargo run`. The child process inherits the caller's environment, UID/GID, filesystem access, and network (via Cargo). This is intentional for a verification harness, but any future codegen bug or unsanitized interpolation would immediately become native code execution. Codegen currently relies on the lexer restricting identifiers to `[A-Za-z0-9_]+`, which blocks direct injection of statements/macros, but there is no second validation gate at the codegen boundary. |
| **Suggestion** | For untrusted inputs (CI jobs, web-wrapped CLI), run the harness inside a sandbox (Linux namespaces/seccomp, `cargo run --offline`, cleared environment, CPU/memory limits). Add an explicit `is_valid_rust_ident()` check in `optic-codegen-rust` before emitting any user-derived name into source. |
| **Status** | open |

---

### 2. No input size bound on source file reads (memory exhaustion DoS)

| Field | Value |
|---|---|
| **Severity** | suggestion |
| **File:Line** | `crates/optic-cli/src/main.rs:282-284` |
| **Description** | `read_source()` loads the entire file into a `String` with no size cap. A multi-gigabyte file (or FIFO) can exhaust memory or block the process. All CLI subcommands that accept a `file` argument use this helper. |
| **Suggestion** | Enforce a configurable maximum source size (e.g. 1–16 MiB for v0) before `fs::read_to_string`, returning a structured diagnostic. Consider `fs::metadata` size check first. |
| **Status** | open |

---

### 3. Transpile output path follows user-controlled input path (path traversal write)

| Field | Value |
|---|---|
| **Severity** | suggestion |
| **File:Line** | `crates/optic-cli/src/main.rs:96-101` |
| **Description** | When `--out` is omitted, transpile writes to `file.with_extension("rs")`, preserving `..` components from the input path. Example: `opticc transpile ../../outside/project/foo.opt` writes `../../outside/project/foo.rs`. This is not privilege escalation on its own (the invoking user must already have write access), but it is exploitable when a wrapper service passes unsanitized user filenames/paths to the CLI without fixing the output location. |
| **Suggestion** | Canonicalize and reject paths that escape a configured root (e.g. `canonicalize()` + prefix check), or require explicit `--out` in automated contexts. Never derive the output path from untrusted input without normalization. |
| **Status** | open |

---

### 4. CLI reads arbitrary filesystem paths without confinement (path traversal read)

| Field | Value |
|---|---|
| **Severity** | suggestion |
| **File:Line** | `crates/optic-cli/src/main.rs:282-284` (all `file: PathBuf` subcommands) |
| **Description** | `read_source()` accepts any path the user supplies, including symlinks (`../../../etc/passwd`) and paths outside the project tree. Content is parsed in memory; while diagnostics emit spans (not source text), error context strings include `file.display()`. In a service deployment, this enables an attacker to probe/read arbitrary readable files by path. |
| **Suggestion** | For automated pipelines, restrict inputs to a chroot/jail or enforce `canonicalize()` + allowed-prefix policy. Do not echo full attacker-controlled paths in stdout/stderr returned to untrusted clients. |
| **Status** | open |

---

### 5. Run harness subprocess inherits full parent environment

| Field | Value |
|---|---|
| **Severity** | suggestion |
| **File:Line** | `crates/optic-cli/src/main.rs:370-375` |
| **Description** | `Command::new("cargo")` is spawned without `.env_clear()` or an allowlist. Environment variables such as `RUSTC_WRAPPER`, `RUSTFLAGS`, `CARGO_TARGET_DIR`, and `PATH` from the parent process affect the verification build. A local attacker who can set the victim's environment before invoking `opticc run` can influence compilation behavior. This is a general subprocess hygiene issue, not shell injection (arguments are passed safely via `.arg()`). |
| **Suggestion** | Use `.env_clear()` and set only required variables (`PATH`, `HOME`, `CARGO_HOME` if needed). Consider `cargo run --offline` to reduce network fetch risk from a fresh temp manifest. |
| **Status** | open |

---

### 6. `optic-runtime` path dependency is not validated at runtime

| Field | Value |
|---|---|
| **Severity** | suggestion |
| **File:Line** | `crates/optic-cli/src/main.rs:354-356`, `361-366` |
| **Description** | The verification harness embeds `optic-runtime = { path = "{runtime}" }` in a generated `Cargo.toml`, where `runtime` is `CARGO_MANIFEST_DIR/../optic-runtime` at build time. The code checks only that `src/lib.rs` exists (`doctor_check`, line 211) but does not canonicalize the path or verify it is the expected crate. A local attacker with write access adjacent to the install tree could replace the directory or plant a symlink before `optic run`, causing Cargo to compile/run attacker-controlled dependency code alongside generated `main.rs`. |
| **Suggestion** | Canonicalize `runtime_crate_path()`, verify it matches an expected prefix or hash of `Cargo.toml`/`lib.rs`, and reject symlinks pointing outside the install root. |
| **Status** | open |

---

### 7. Verification failure logs raw Cargo stderr (path/build metadata leakage)

| Field | Value |
|---|---|
| **Severity** | nit |
| **File:Line** | `crates/optic-cli/src/main.rs:376-378` |
| **Description** | On harness failure, the full `cargo` stderr is written to stderr via `eprintln!`. This can include absolute temp paths, toolchain details, and dependency resolution messages. Low risk for local dev; undesirable if CLI output is forwarded to untrusted parties or centralized logging. |
| **Suggestion** | Emit a short user-facing error and log verbose build output only behind a `--verbose` flag. Redact absolute paths in production logging. |
| **Status** | open |

---

### 8. Diagnostic JSON echoes resolver messages verbatim in evidence

| Field | Value |
|---|---|
| **Severity** | nit |
| **File:Line** | `crates/optic-diagnostics/src/lib.rs:109-118` |
| **Description** | `resolve_diag` places the full resolver `message` into both `rule` and `evidence.name`. Today messages are compiler-generated (`unknown optic`, etc.), but if future resolver messages incorporate user-defined identifiers or external data without sanitization, `--json` output could leak unexpected content into CI artifacts. |
| **Suggestion** | Keep `evidence` structured (typed fields) rather than copying free-form messages; sanitize/limit string lengths in JSON diagnostics. |
| **Status** | open |

---

## Areas Reviewed — No Open Issues Found

### Parser / input sanitization
- Lexer restricts identifiers to ASCII alphanumerics and `_` (`lexer.rs:236-242`), which blocks direct injection of Rust tokens (semicolons, quotes, macros) into codegen output.
- Parser recovery now advances past sync tokens (`skip_until_sync` consumes the sync token), fixing the prior infinite-loop DoS on malformed fn bodies and typed lets.
- `format_hir_expr` / `emit_hir_expr_rust` only interpolate numeric literals and validated identifiers for the supported expression subset; fallback arms return safe placeholders (`"v"`).

### Command injection in run harness
- `cargo` is invoked via `std::process::Command` with discrete `.arg()` calls; the manifest path is a `PathBuf` from `tempfile`, not a shell string. No `sh -c` usage. **No command injection vector identified.**

### Temp file handling (`/tmp/optic_verify`)
- Implementation uses `tempfile::tempdir()` (`main.rs:359`), which provides randomized, user-private directories cleaned up on drop. This is **strictly better** than the fixed `/tmp/optic_verify` path still mentioned in `PLAN.md:227` (predictable shared paths would enable symlink/race attacks between users). **No open temp-file vulnerability** in the current code.

### Sensitive data in logs/diagnostics
- `Span` serialization contains only `SourceId` + byte offsets, not source text (`span.rs:13-18`).
- `check --json` does not embed raw source content.
- No secrets (tokens, keys) observed in diagnostic catalogs or CLI output.

---

## Fixed in This Iteration (Not Reported as Open)

| Issue | Resolution |
|---|---|
| Parser infinite loop / hang (DoS) | `skip_until_sync` consumes sync token; fn-body and typed-let recovery paths fixed (`parser.rs`) |
| Predictable shared temp dir (if `/tmp/optic_verify` had been used) | Replaced by `tempfile::tempdir()` in harness |

---

## Review Metadata

- **Commands run:** `git log`, `grep` across security-relevant patterns; targeted file reads of `optic-cli/src/main.rs`, `optic-codegen-rust/src/lib.rs`, `optic-syntax` lexer/parser, `optic-diagnostics`, `optic-hir`, `optic-cgir`, `optic-opt`.
- **Runtime verification:** Limited by environment (`No file descriptors available`); static analysis used for codegen injection assessment.
- **Bug count:** 0 open  
- **Suggestion count:** 6 open  
- **Nit count:** 2 open