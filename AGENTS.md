# AGENTS.md — plugin-flaresolverr

> **Start at the roadmap and board on [lemonfiber.app](https://lemonfiber.app),
> rendered from the report of where every unreleased version stands. Then the
> rules** every repository shares:
> [working in the repositories](https://github.com/lemonfiber/spec/blob/main/50-governance/working-in-the-repositories.md)
> and [the rules for agents](https://github.com/lemonfiber/spec/blob/main/50-governance/ai-contributors.md).
> This file holds only what is true of this repository.

## What this repo is

FlareSolverr as a first-party lemonfiber plugin: `plugin.toml` and the adapter
that speaks the `indexer.proxy@1` contract in front of FlareSolverr, asking it
through its API. The adapter is the Rust crate at the root, built on
`lemonfiber-adapter-kit` from the core at the commit `Cargo.toml` pins, into the
image the `Dockerfile` builds. `release.yml` publishes that image from every
commit on `main` and from the train's tag. `plugin.toml` runs FlareSolverr alone,
and its claim's recordings are FlareSolverr's own answers. The
[bundle contract](https://github.com/lemonfiber/spec/blob/main/20-architecture/contracts/bundle.md)
says how the catalogue pins this repository and how lemonfiber carries it.

## The rules you cannot break

- **The plugin id is the service's id** (`ARCH-R266`). The container, the
  configuration directory, the settings and the journal keep their names.
- **One service, speaking no contract**, while the bundle assembles it. It
  declares `replaces` naming its own id, which is how it takes the id, the port
  and the hostname the bundle reserves for it (`ARCH-R222`).
- **Its claims' recordings sit under `recordings/flaresolverr/`**, and its starting
  files, where it has any, under `config/flaresolverr/` (`ARCH-R271`).
- **Every service declares `reaches` and `asks_for`** (`ARCH-R270`).
- **No Python here.** The reader is spec's `first-party-plugin` workflow's.
- **No `unwrap`, `expect`, `panic` or indexing** outside tests. The lints in
  `Cargo.toml` are the core's.
- **FlareSolverr runs fenced, and the fence is the boundary.** Its service says
  `network = { fenced = true }`, so it runs inside the fence lemonfiber writes
  beside it, whose rules refuse every private address and the host's own.
  `tests/manifest.rs` refuses a manifest whose `indexer.proxy` service is not
  fenced.
- **The adapter screens every address first.** It is `http` or `https`, carries
  no user or password, names its host by a name rather than a number, and every
  address that name resolves to passes `lemonfiber_outward`. Anything else is
  refused naming the reason, never the address, before FlareSolverr is asked, and
  what is handed over is the address as the screen read it. The browser follows
  redirects and resolves names again, so the screen is a first refusal and never
  the protection.
- **The adapter answers inside the contract's deadline.** FlareSolverr is given
  `SOLVE_WITHIN` to fetch a page and the adapter waits `ANSWER_WITHIN` for it,
  both under the 60 seconds `indexer.proxy@1` allows.
- **The core's crates move with the train.** The `rev` in `Cargo.toml` is the
  core commit `targets.toml` names, the same for every `lemonfiber-*` crate.
- **Moving FlareSolverr's digest** means recording every response again against
  the new image in the same change, moving `RELEASE` and `DIGEST` in
  `src/adapter.rs` with it, and committing the `proofs.json` the checks write.
