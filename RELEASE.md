# Releasing

How to cut a release: publish the crate to crates.io and update the live app. Manual process, run from `main` after the work is merged. There is no tag-triggered publish workflow.

Two destinations:

- The Rust crate to [crates.io](https://crates.io/crates/guitar-tab-generator). The `documentation` link in `Cargo.toml` points at docs.rs, which builds once the version is live.
- The live app at [noahbaculi.github.io](https://github.com/noahbaculi/noahbaculi.github.io), which vendors the `wasm-pack` output (not an npm dependency). See step 7.

## 1. Pre-Flight Checks

Run the gates CI runs (`.github/workflows/rust_build_and_test.yml`). All must pass.

> [!WARNING]
> `cargo publish` can't be undone. A version, once on crates.io, can only be yanked, not replaced.

```shell
cargo build --examples --benches
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo +1.86 check --all-targets          # MSRV from Cargo.toml
wasm-pack test --node -- --test wasm_boundary
wasm-pack build --target web --out-dir pkg/wasm_guitar_tab_generator
```

Regenerate the `.d.ts` snapshot if the public TypeScript surface changed, then commit it:

```shell
sed '/^export type InitInput/,$d' \
  pkg/wasm_guitar_tab_generator/guitar_tab_generator.d.ts > tests/snapshots/wasm.d.ts
```

## 2. Version and Docs

- Bump `version` in `Cargo.toml` (semver - a public API break is a major bump).
- Add a dated `CHANGELOG.md` entry in the existing header format (`## 2.1.0 -- 2026-06-05`), with `Breaking changes` / `Added` / `Fixed` sections.
- For a breaking change, add migration steps to `MIGRATION.md` and an ADR under `docs/adr/` for any decision worth recording.
- Update `README.md` if the version is referenced anywhere consumer-facing.

Commit the bump and changelog together (for example `chore(release): 2.1.0`).

## 3. Validate the Package

Inspect the tarball, then dry-run the exact build crates.io will run:

```shell
cargo package --list      # files in the published tarball
cargo publish --dry-run
```

> For the first publish, confirm the name `guitar-tab-generator` is free on crates.io. The first `cargo publish` claims it.

## 4. Publish

```shell
cargo login        # first time only - token from https://crates.io/settings/tokens
cargo publish
```

## 5. Tag and GitHub Release

```shell
git tag -a v2.1.0 -m "v2.1.0"
git push origin v2.1.0
```

Create a GitHub Release from the tag. Paste the matching `CHANGELOG.md` section as the notes. Optionally attach the built `guitar_tab_generator_bg.wasm`.

## 6. Verify

- crates.io page shows the new version.
- docs.rs build succeeds, so the `documentation` link resolves.

## 7. Update the Live Website

The deployed app vendors the `wasm-pack` output under `assets/wasm_guitar_tab_generator/` in the [noahbaculi.github.io](https://github.com/noahbaculi/noahbaculi.github.io) repo. It pulls from neither crates.io nor npm, so it updates only when you copy the rebuilt files in.

- Build: `wasm-pack build --target web --out-dir pkg/wasm_guitar_tab_generator`.
- On a branch off `staging`, replace `assets/wasm_guitar_tab_generator/` with the generated files. `wasm-pack` regenerates `package.json` with the version from `Cargo.toml`.
- Port `assets/js/guitartab.js` if the API changed. A major bump is a rewrite, not a file swap. Map it through `CHANGELOG.md` and `MIGRATION.md`.
- Smoke-test locally (checklist in `examples/wasm.html`), then open a PR into `staging`.

## First Release

`Cargo.toml` already reads `2.0.0`, but nothing is tagged or published, so the first run skips the bump in step 2. Cut straight from `2.0.0`: run the pre-flight checks, `cargo publish`, tag `v2.0.0`, create the Release from the existing 2.0.0 changelog entry.

## Note on npm

The wasm package isn't published. Both consumers build or vendor from source (the demo in `examples/wasm.html` and the live website both import the `wasm-pack --target web` output by relative path), so neither needs a registry. A static GitHub Pages site with no bundler can't `npm install` anyway.

Publish to npm only when a consumer with a build step wants to install instead of vendor. At that point:

```shell
wasm-pack build --target bundler --out-dir pkg/wasm_guitar_tab_generator
wasm-pack publish        # uses the package name from the generated package.json
```

Pick the package name (and any npm scope) deliberately - it becomes the public install name.
