# V1 Release Checklist

## Environment Setup

### Recommended Toolchains
- **Windows**: MSVC toolchain (`rustup default stable-x86_64-pc-windows-msvc`) with Visual Studio Build Tools 2022
- **Alternative**: MinGW-w64 via WinLibs or MSYS2 (`rustup default stable-x86_64-pc-windows-gnu`)

### Known Environment Issue
If you see `STATUS_ENTRYPOINT_NOT_FOUND (0xC0000139)` when running `cargo test --lib`, this is caused by mixing multiple MinGW CRT variants on the same system (e.g., WinLibs MCF + LLVM-MinGW UCRT). The test binary links against proc-macro DLLs that may use a different CRT than the binary itself.

**Fix**: Uninstall all MinGW variants, keep only one (recommend WinLibs UCRT or MSVC). Or use MSVC entirely.

## Build Commands

```bash
# Quick check (recommended for development)
cargo check

# Compile tests (verifies test logic compiles)
cargo test --lib --no-run

# Run tests (requires WebView2 Runtime + clean MinGW/MSVC)
cargo test --lib

# Frontend
npm run build        # tsc + vite build

# Full production build (generates installer)
npm run tauri build
```

## Release Artifacts

| Artifact | Path |
|----------|------|
| NSIS Installer | `src-tauri/target/release/bundle/nsis/Game Agent Studio_x64-setup.exe` |
| MSI Installer | `src-tauri/target/release/bundle/msi/Game Agent Studio_x64_en-US.msi` |
| Portable EXE | `src-tauri/target/release/buildgameagent.exe` (+ `WebView2Loader.dll`) |

## Pre-Release Verification

- [ ] `cargo check` passes with 0 errors
- [ ] `cargo test --lib --no-run` compiles successfully
- [ ] `npm run build` (tsc + vite) passes
- [ ] All security regression tests compile
- [ ] Command manifest test passes (HANDLER_NAMES == ALLOWED_COMMANDS == generate_handler![])
- [ ] API key not present in any frontend store or log
- [ ] Export path restricted to app data directory
- [ ] No shell/fs/dialog plugin permissions

## Known Limitations (V1)

- **No streaming LLM responses** — all API calls are synchronous batch
- **Keyword-only RAG search** — no vector/embedding retrieval yet
- **No workflow editor** — workflows are defined as static Rust constants
- **Template exports deferred** — Godot/Ren'Py/Phaser project export not implemented
- **OS Keychain**: Windows/macOS use native credential manager via `keyring` crate. Linux uses Secret Service; if DBus unavailable, falls back to `LocalEncryptedSecretStore` (AES-256-GCM with hostname-derived key). Migration from legacy encrypted key to keychain happens on first launch after upgrade.
- **`cargo test --lib` may fail on MinGW-mixed systems** — see Environment Setup section above

## Migration from v0.1.x

- API key must be re-configured (old plaintext column migrated to encrypted)
- Export paths changed; old export scripts that specified custom output_dir will not work
