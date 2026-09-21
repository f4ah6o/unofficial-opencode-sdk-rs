# Publishing

The crates.io package name is `unofficial-opencode-sdk`. The Rust import path is
`unofficial_opencode_sdk`.

Publishing is intentionally separated from normal CI. The repository does not
store a long-lived crates.io API token.

## Pre-publish checks

Run these before every release:

```sh
cargo fmt --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --no-deps --all-features
cargo package --list
cargo publish --dry-run
```

Normal GitHub Actions CI runs the same package dry-run and also verifies the
declared Rust 1.85 MSRV.

## First crates.io publication

As of the current crates.io Trusted Publishing design, a Trusted Publisher can
only be configured after the crate exists on crates.io. Therefore the first
`0.1.0` publication is the one bootstrap exception: publish it manually with a
short-lived/scoped crates.io credential after reviewing the package produced by
`cargo publish --dry-run`.

Do not add that credential to this repository or to GitHub Actions.

No publication is performed merely by merging this repository preparation.

## Trusted Publisher configuration

After the first publication, configure the crate's Trusted Publisher on
crates.io with these exact claims:

- Provider: GitHub Actions
- Crate: `unofficial-opencode-sdk`
- GitHub owner: `f4ah6o`
- Repository: `unofficial-opencode-sdk-rs`
- Workflow file: `publish.yml`
- Environment: `release`

Also create/configure the GitHub `release` environment. Repository/environment
protection rules can be used to require explicit approval before a production
publish.

Once the Trusted Publisher is working, enable crates.io's
Trusted-Publishing-Only setting if desired so long-lived/manual credentials
cannot publish future versions.

## Subsequent releases

1. Update the version in `Cargo.toml`.
2. Update `CHANGELOG.md`.
3. Merge only after CI and contract checks are green.
4. Create a GitHub Release using tag `v<version>`, for example `v0.1.1`.
5. Publishing the GitHub Release triggers `.github/workflows/publish.yml`.
6. The workflow verifies that the release tag exactly matches the Cargo package
   version, runs tests and `cargo publish --dry-run`, requests a short-lived
   crates.io token through GitHub OIDC, and only then runs `cargo publish`.

Prereleases are deliberately ignored by the publishing job.
