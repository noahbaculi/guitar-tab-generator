## Agent skills

### Issue tracker

Issues and PRDs live as markdown files under `.scratch/<feature-slug>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Canonical triage roles (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`) map 1:1 to label strings. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Releases

`RELEASE.md` is the manual release process: publish to crates.io, tag, create the GitHub release, and copy the `wasm-pack` build into the website with a PR into its `staging` branch. A crates.io version can only be yanked, never replaced. Run `cargo publish`, push tags, create releases and open the website PR only when the user asks.
