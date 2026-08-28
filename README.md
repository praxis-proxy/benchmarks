# Praxis Benchmarks

`praxis-bench` is the shared benchmarking and performance-testing tool for
[Praxis](https://github.com/praxis-proxy/praxis). It load-tests a Praxis (or
compatible) proxy image and produces comparison reports against Envoy, NGINX,
and HAProxy baselines, so any repository with a standard proxy build (praxis,
ai, experimental) can measure it the same way.

The proxy under test is treated as an opaque container image: plug in your
image with `--image` and get results. The tool does not depend on Praxis
source.

## Usage

Run the CLI directly:

```console
cargo run -p praxis-bench -- --image ghcr.io/praxis-proxy/praxis:latest
```

Or use the container runner, which bundles the `docker` CLI and the
`vegeta`/`fortio` load generators. Share the host Docker socket and network so
it can start proxy containers and reach them:

```console
docker run --rm --network host \
  -v /var/run/docker.sock:/var/run/docker.sock \
  ghcr.io/praxis-proxy/benchmarks --image ghcr.io/praxis-proxy/praxis:latest
```

Subcommands:

```console
praxis-bench                 # run benchmarks, write a report
praxis-bench visualize ...   # render an SVG chart from a report
praxis-bench compare A B     # compare two reports for regressions
```

A full run needs a Docker (or Podman) daemon and the `vegeta` and `fortio`
load generators on PATH; both are bundled in the container image.

## What Is Enforced

This repository follows the praxis-proxy
[conventions](https://github.com/praxis-proxy/conventions): the same
machine-enforced quality standards used across the organization.

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

## Development

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
