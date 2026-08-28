# Praxis Benchmarks

Shared benchmarking and performance-testing tooling for
[Praxis](https://github.com/praxis-proxy/praxis), intended to be reusable
across repositories with a standard proxy build (praxis, ai, experimental).

This repository has adopted the praxis-proxy
[conventions](https://github.com/praxis-proxy/conventions): the same lint
configuration, quality gates, supply-chain checks, and release pipeline used
across the organization. The benchmark harness itself has not landed here yet;
the `benchmarks-probe` crate is a temporary placeholder that keeps every gate
verifiable against real code until the harness is extracted.

## What Is Enforced

- **Lints**: ~200 rustc/clippy/rustdoc lints at deny, including no unchecked
  arithmetic, no `as` casts, no `unwrap`/`panic`, exhaustive enum matching, and
  documentation on every item (public and private)
- **Testing**: unit + integration tests required; 90% line / 80% region
  coverage floor; mutation testing; property-based testing conventions
- **Supply chain**: `cargo audit` + `cargo deny` with pinned registries and a
  license allowlist
- **Reviewability**: PRs capped at 750 added production lines, with required
  descriptions, conventional commits, DCO sign-off, and signed commits
- **Everything else**: markdown, TOML, shell, spelling, and workflow files are
  linted too

## Quickstart

Install the [requirements](docs/development.md), then:

```console
make all            # build + fmt + lint + test + audit
make help           # every available target
```

## Documentation

| Document | Contents |
| --- | --- |
| [conventions.md](docs/conventions.md) | Coding style, testing, type design, lint policy |
| [development.md](docs/development.md) | Requirements, build/test/coverage commands |
| [enhancements](https://github.com/praxis-proxy/enhancements) | Proposal lifecycle for larger changes |
| [release.md](docs/release.md) | Versioning, tagging, release pipeline |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contributor entry point and PR gates |
