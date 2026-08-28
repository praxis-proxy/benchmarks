//! `praxis-bench`: a generic proxy benchmarking tool.
//!
//! Load-tests a Praxis (or compatible) proxy image supplied via `--image`
//! and produces comparison reports against Envoy, NGINX, and `HAProxy`
//! baselines. The proxy under test is treated as an opaque container, so
//! any build that serves the Praxis config schema can be benchmarked.

#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::exit,
    clippy::indexing_slicing,
    clippy::min_ident_chars,
    clippy::mod_module_files,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::shadow_unrelated,
    clippy::single_char_lifetime_names,
    clippy::struct_field_names,
    clippy::unused_result_ok,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::wildcard_enum_match_arm,
    reason = "command-line benchmarking tool"
)]
#![allow(let_underscore_drop, reason = "command-line benchmarking tool")]

use clap::Parser as _;

mod bench;

// -----------------------------------------------------------------------------
// Main
// -----------------------------------------------------------------------------

/// Parse CLI arguments and dispatch to the benchmark runner.
fn main() {
    let args = bench::Args::parse();
    bench::run(args);
}

// -----------------------------------------------------------------------------
// Tracing Setup
// -----------------------------------------------------------------------------

/// Initialize tracing with the given default level.
///
/// Respects `RUST_LOG` if set, otherwise falls back to `default_level`.
/// Set `PRAXIS_BENCH_LOG_FORMAT=json` for structured JSON output.
pub(crate) fn init_tracing(default_level: &str) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_level));

    let json = std::env::var("PRAXIS_BENCH_LOG_FORMAT").is_ok_and(|v| v.eq_ignore_ascii_case("json"));

    if json {
        tracing_subscriber::fmt().json().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    }
}
