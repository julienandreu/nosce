# nosce

Git submodule watcher that generates daily changelogs and architecture documentation using Claude.

## Development Commands

```bash
# Build (from repo root — workspace)
cargo build --release          # Optimized binary → target/release/nosce
cargo build                    # Debug binary → target/debug/nosce
cargo test                     # Run all tests
cargo clippy                   # Lint

# Frontend
cd frontend && npm install     # Install deps
cd frontend && npm run build   # Production build → mcp-server/static/
cd frontend && npm run dev     # Vite dev server (hot reload, port 5173)
cd frontend && npm run lint    # ESLint
cd frontend && npm run typecheck  # TypeScript strict check

# Run
./target/release/nosce --output-dir ~/.nosce/output serve        # Web UI
./target/release/nosce --output-dir ~/.nosce/output serve -d     # Daemon
./target/release/nosce stop                                       # Stop daemon
```

## Architecture

```
Cargo.toml              ← Workspace root (resolver = "2")
├── mcp-server/         ← Rust crate: binary "nosce"
│   ├── src/main.rs     ← CLI entry (clap): mcp | serve | stop
│   ├── src/config.rs   ← Profile definitions + nosce.yml loader
│   ├── src/server.rs   ← MCP tools + resources (rmcp)
│   ├── src/web.rs      ← HTTP API + static file serving (axum)
│   └── src/fs_ops.rs   ← Non-blocking filesystem operations
├── frontend/           ← Preact SPA (TypeScript, Tailwind, Catppuccin)
├── .claude/skills/     ← /sync and /docs skill definitions
└── nosce.yml           ← Default configuration
```

## Key Patterns

- **Error handling**: Use `anyhow` with `.context()` for all fallible operations. No `.unwrap()`.
- **Async**: Tokio runtime, non-blocking fs via `fs_ops.rs`.
- **CLI**: Clap derive macros with env var fallbacks.
- **Web**: Axum with tower-http middleware (CORS, tracing).
- **Frontend**: Preact + TypeScript strict mode + Tailwind CSS. ESLint strict + stylistic.
- **Profiles**: Role-based views (engineer, product, sales) defined in nosce.yml.

## Testing

```bash
cargo test                     # Rust tests
cd frontend && npm test        # Frontend tests (if configured)
```

## Release

Releases are built via GitHub Actions on tag push (`v*`). The workflow builds for:
- macOS: x86_64, aarch64
- Linux: x86_64 (musl), aarch64

Binary naming: `nosce-{version}-{arch}-{os}.tar.gz`
