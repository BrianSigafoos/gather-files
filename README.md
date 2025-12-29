# gather-files (`gf`)

**Codebase → Clipboard → AI.**

Gather your source files (README first), stitch them with headers, and copy to clipboard—ready to paste into the smartest "pro" AI models.

## Why?

The best "pro" AI models (Claude Pro, ChatGPT Pro, etc) are often web-UI only. To use them with your codebase, you need to paste context manually. `gf` makes that trivial:

- **One command** gathers your entire repo or a curated subset
- **README first**—AI loves context upfront
- **Skips noise**: `.git`, `node_modules`, `target`, binaries
- **Reports stats**: character count + timing for token budgeting

## Install

```bash
curl -LsSf https://gf.bfoos.net/install.sh | bash
```

Or with Rust:

```bash
cargo install --git https://github.com/BrianSigafoos/gather-files
```

## Usage

```bash
gf              # gather entire repo (git root)
gf <path>       # gather a specific directory or file
gf <preset>     # gather files defined in .gather-files.yaml

# Options
gf --config path/to/config.yaml
```

## Configuration (`.gather-files.yaml`)

Presets let you gather curated file sets with glob patterns:

```yaml
version: 1
presets:
  my_feature:
    base: .
    include:
      - "doc/plan/feature-plan.md"
      - "app/controllers/feature/**/*.rb"
      - "app/javascript/controllers/feature_controller.js"
    exclude:
      - "app/controllers/feature/internal/**"
```

- `base` (optional): scopes glob patterns; defaults to repo root
- `include`: glob patterns to gather (required, at least one)
- `exclude`: patterns to skip (optional)

Run `gf my_feature` to gather just those files. Errors if no files match.

## Development

```bash
cargo fmt
cargo test
cargo run -- --help
```

The clipboard helper tries `pbcopy`, `wl-copy`, `xclip`, then `clip`. Tests cover path + preset collectors.

## Releasing

Uses [`cargo-release`](https://github.com/crate-ci/cargo-release):

```bash
cargo install cargo-release
cargo release patch --execute
```

GitHub Actions builds binaries and publishes the release automatically.
