## Agent skills

### Domain docs

Single-context: one `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Releases

`RELEASE.md` is the manual release process: publish to crates.io, tag, create the GitHub release, and copy the `wasm-pack` build into the website with a PR into its `staging` branch. A crates.io version can only be yanked, never replaced. Run `cargo publish`, push tags, create releases and open the website PR only when the user asks.
