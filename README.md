# gather-files (`gf`)

`gf` is a Rust CLI that gathers source files (README first), stitches them together with headers, copies the blob to your clipboard, and reports stats (character count + elapsed time).

## Install

```bash
curl -LsSf https://gf.bfoos.net/install.sh | bash
# or:
cargo install --git https://github.com/BrianSigafoos/gather-files
```

Docs + a copy/paste installer live at https://gf.bfoos.net.

## Usage

```bash
gf              # gather entire repo (git root)
gf <path>       # gather a specific directory or file
gf <preset>     # gather files defined in .gather-files.yaml

# Options
gf --config path/to/config.yaml
```

Behaviors:

- README files near the requested path are promoted to the top.
- Directories like `.git`, `target`, and `node_modules` are skipped automatically.
- Output is formatted with header, path, blank line, file contents

## Configuration (`.gather-files.yaml`)

When you call `gf preset_name`, the CLI looks up the preset in `.gather-files.yaml` (relative to the repo root unless you override `--config`). Presets let you gather arbitrary glob patterns from anywhere in the repo.

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

- `base` (optional) scopes the glob patterns; omit it to use the repo root.
- `include` lists glob patterns (using `**` etc.). At least one pattern is required.
- `exclude` removes matches relative to `base` (optional).

`gf foo` errors if a preset pattern matches nothing to make mistakes obvious.

## Development

```bash
cargo fmt
cargo test
cargo run -- --help
```

The clipboard helper tries `pbcopy`, `wl-copy`, `xclip`, then `clip`. Make sure one of those exists on your system. Tests cover the path + preset collectors; add new tests for additional behaviors.

## Releasing

This repo uses [`cargo-release`](https://github.com/crate-ci/cargo-release). Install it once with:

```bash
cargo install cargo-release
```

Then run the VS Code "Release Patch" task (or `cargo release patch --execute`) to bump the version, tag, and push. GitHub Actions will build binaries and publish the release automatically.
