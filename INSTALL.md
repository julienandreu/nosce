# Installing nosce

## Quick Install

Download and install the latest release:

```bash
curl -fsSL https://raw.githubusercontent.com/julienandreu/nosce/main/install.sh | sh
```

This installs to `~/.local/bin/nosce`. Set `NOSCE_INSTALL_DIR` to change the location:

```bash
NOSCE_INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/julienandreu/nosce/main/install.sh | sh
```

## Building from Source

### Prerequisites

| Tool | Install |
|------|---------|
| Rust 1.70+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js 18+ | `brew install node` or [nodejs.org](https://nodejs.org) |

### Build

```bash
git clone https://github.com/julienandreu/nosce.git
cd nosce

# Build the Rust binary
cargo build --release

# Build the frontend
cd frontend
npm install
npm run build
cd ..
```

The binary is at `target/release/nosce`.

## Docker

```bash
docker build -t nosce .
docker run -v ~/.nosce/output:/data -p 3000:3000 nosce
```

Open http://localhost:3000.

## Configuration

Create `~/.nosce/output` for the default output directory:

```bash
mkdir -p ~/.nosce/output
```

Copy and edit `nosce.yml` to configure profiles, input paths, and output paths. See the [README](README.md#configuration) for full details.

## Verify Installation

```bash
nosce --version
nosce --help
```

## Troubleshooting

### `nosce: command not found`

Ensure the install directory is in your PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Add this to your `~/.zshrc` or `~/.bashrc` to make it permanent.

### Build fails with missing `musl-dev`

On Linux, install musl development headers:

```bash
# Debian/Ubuntu
sudo apt install musl-tools

# Alpine
apk add musl-dev
```

### Frontend build fails

Ensure Node.js 18+ is installed:

```bash
node --version  # Should be v18 or higher
```
