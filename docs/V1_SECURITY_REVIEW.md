# V1 Security Review

## API Key Protection

| Layer | Mechanism | Status |
|-------|-----------|--------|
| Frontend | `get_model_config` returns `ModelConfigPublic` (only `has_api_key` + `masked_api_key`) | ✅ |
| Frontend | `LlmRequest` has no `api_key` field | ✅ |
| Storage | AES-256-GCM via `LocalEncryptedSecretStore`, key derived from hostname+SHA-256 | ✅ |
| Error | All error paths through `sanitize_error()` — redacts Bearer/Authorization/api_key/sk-* | ✅ |
| Export | `export_markdown`/`export_json` exclude `qa_review` and `system_internal` memory | ✅ |
| RAG | Chunk excerpts sanitized before prompt injection; UI excerpts sanitized | ✅ |
| OS Keychain | `KeychainSecretStore` using `keyring` crate (Windows Credential Manager / macOS Keychain / Linux Secret Service) | ✅ v0.2.0 |
| Fallback | `LocalEncryptedSecretStore` (AES-256-GCM) if OS keychain unavailable | ✅ |

### SecretStore Architecture

| Platform | Implementation |
|----------|---------------|
| Windows | Credential Manager via `keyring` crate (DPAPI-backed) |
| macOS | Keychain via `keyring` crate |
| Linux | Secret Service via `keyring` crate; falls back to `LocalEncryptedSecretStore` if DBus unavailable |

### Migration Strategy
1. On first launch after upgrade, detect old `encrypted_api_key` in SQLite
2. Decrypt via `LocalEncryptedSecretStore`, store plaintext in OS keychain
3. Replace `encrypted_api_key` with marker `"keychain_stored"` (unusable without keychain)
4. If migration fails (keychain locked/unavailable): the old encrypted key is preserved and runtime falls back to LocalEncryptedSecretStore
5. User may need to re-configure API key if keychain is unavailable on their platform

### Risk Analysis
- V1 (before v0.2.0): key derivable by any process with hostname access on same machine
- V1 (v0.2.0+): key stored in OS credential manager, accessible only to same user/process
- Linux headless: falls back to V1 behavior if DBus/Secret Service unavailable
- Migration is best-effort; failure does not crash the app

## Tauri Permissions

| Item | State |
|------|-------|
| `capabilities/default.json` | `permissions: []` — no plugin permissions |
| Shell | Not in Cargo.toml, not registered |
| FS | Not in Cargo.toml, not registered |
| Dialog | Not in Cargo.toml, not registered |
| Custom commands | 40 commands in `generate_handler![]` |
| Command manifest | `HANDLER_NAMES` == `ALLOWED_COMMANDS` == `generate_handler![]` verified by `include_str!` test |

## CSP Policy

| Mode | `connect-src` |
|------|---------------|
| Production | `'self' ipc:` |
| Development | `'self' ipc: http://localhost:1420 ws://localhost:1421` |

## Network Boundary

| Rule | Production | Debug |
|------|-----------|-------|
| `https://` public domain | ✅ Allowed | ✅ Allowed |
| `http://` public domain | ❌ Rejected | ❌ Rejected |
| `http://localhost` | ❌ Rejected | ✅ Allowed |
| `http://127.0.0.1` | ❌ Rejected | ✅ Allowed |
| `http://[::1]` | ❌ Rejected | ✅ Allowed |
| `https://localhost` | ❌ Rejected | ❌ Rejected |
| `https://127.0.0.1` | ❌ Rejected | ❌ Rejected |
| `file://` / `ftp://` | ❌ Rejected | ❌ Rejected |
| Private IP (10/8, 172.16/12, 192.168/16) | ❌ Rejected | ❌ Rejected |
| Link-local (169.254, fe80::/10) | ❌ Rejected | ❌ Rejected |
| `localhost.evil.com` | ❌ Rejected | ❌ Rejected |

## Request Limits

| Limit | Value |
|-------|-------|
| reqwest timeout | 60 seconds |
| Max messages per request | 20 |
| Max total chars per request | 40,000 |
| Max tokens (capped at save + runtime) | 32,768 |
| RAG query max chars | 500 |
| RAG query max words | 20 |
| RAG result limit (clamped) | 1..=20 |

## File Export Boundaries

- Exports go to `{data_dir}/game-agent-studio/exports/` only
- Path canonicalize on parent directory (file doesn't exist yet)
- Post-write verification: canonicalize file, verify within exports dir, delete if escaped
- Filename sanitization: strips path separators, control chars, Windows reserved names
- Export content excludes `qa_review`, `system_internal` memory types

## Event Audit

| Event | actor | severity | Contains |
|-------|-------|----------|----------|
| `workflow_start` | system | info | workflow_type, task |
| `step_start` | system | info | step_key, agent, agent_role |
| `step_complete` | system | info | step_key, tokens |
| `step_failed` | system | error | step_key, sanitized error |
| `workflow_complete` | system | info | correlation_id |
| `workflow_failed` | system | error | sanitized error |
| `output_accepted` | user | info | message_id, correlation_id |
| `output_rejected` | user | info | message_id, correlation_id |
| `output_edited` | user | info | message_id, correlation_id |
| `memory_saved` | system | info | step_key, agent |
| `export_created` | system | info | export_id, file_path |
| `proposal_created` | system | info | proposal_id, type, risk_level |
| `proposal_reviewed` | user | info | proposal_id, old→new status |
| All events | — | — | `event_data` sanitized before INSERT |

## Self-Iteration Safety

- Proposals generated from event analysis only
- Accept/Reject only updates `status` + writes audit event
- No automatic code/configuration/file modifications
- Code/Prompt/Safety/Export/DataModel types default `requires_human_approval=true`
- Status transitions validated: draft→proposed→accepted/rejected→implemented/superseded
- Invalid transitions return error

## Memory / Version Integrity

- `save_project_memory` validates: memory_type allowlist, layer (L1-L4), scope (project/session/global), confidence (0.0-1.0), version (≥1)
- Layer/scope combo validated (e.g., L1 only allows `session` scope)
- Updates write to `memory_versions` (old_value/new_value) before UPDATE
- `memory_versions` INSERT + `project_memory` UPDATE in same transaction
- Returns original `created_at`, not update time

## RAG Audit Chain

| Step | Recorded |
|------|----------|
| Document created | `documents` row with id/project/title/type |
| Document chunked | `document_chunks` rows with metadata (source/provenance/content_hash) |
| Search executed | `retrieval_runs` row (query/strategy/duration_ms) |
| Hits recorded | `retrieval_hits` rows (score/rank/chunk_id/used_by_agent) |
| Agent uses hits | `used_by_agent` populated with run_id/step_key/agent_name JSON |
| Agent step | `agent_steps.input_json` contains `retrieval_run_id` + `retrieval_hits[]` |
| Excerpts to prompt | Sanitized via `sanitize_error()` before injection |
| UI display | Excerpts sanitized via `sanitize_error()` before response |

## Known Limitations

- **Keyword + hybrid (vector cosine similarity) RAG search** — V1 supports both keyword and vector retrieval
- No streaming LLM responses
- `LocalEncryptedSecretStore` fallback on Linux when DBus/Secret Service unavailable
- `cargo test --lib` runtime requires clean MinGW or MSVC + WebView2 Runtime
- No template project exports (Godot/Ren'Py/Phaser deferred to V2)
