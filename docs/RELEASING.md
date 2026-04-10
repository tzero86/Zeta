# Releasing Zeta

Zeta releases are created automatically by GitHub Actions when you push a tag
matching `v*`.

## Release flow

1. Ensure `main` is in the state you want to release.
2. Make sure these pass locally if possible:
   - `cargo test --workspace`
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. Create and push a tag.

## Stable release

```bash
git tag v0.2.0
git push origin v0.2.0
```

## Pre-release

Any tag containing `-dev`, `-rc`, `-beta`, or `-alpha` is marked as a GitHub
pre-release automatically.

Examples:

```bash
git tag v0.2.0-dev.1
git push origin v0.2.0-dev.1
```

```bash
git tag v0.2.0-rc.1
git push origin v0.2.0-rc.1
```

## What the workflow publishes

For each release tag, GitHub Actions builds:

- Linux x86_64
- Windows x86_64

Published release assets:

- `zeta-linux-x86_64.tar.gz`
- `zeta-linux-x86_64.tar.gz.sha256`
- `zeta-windows-x86_64.zip`
- `zeta-windows-x86_64.zip.sha256`

GitHub-generated release notes are enabled automatically.

## Notes

- The workflow uses `cargo build --release --locked`, so `Cargo.lock` must be
  committed and up to date.
- If a release tag is pushed by mistake, delete the tag locally and remotely,
  then delete the GitHub Release manually if it was already created.

## Deleting a mistaken tag

```bash
git tag -d v0.2.0-dev.1
git push origin :refs/tags/v0.2.0-dev.1
```
