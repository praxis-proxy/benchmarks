//! Proxy configuration trait and built-in implementations.

mod envoy;
mod haproxy;
mod nginx;
mod praxis;

use std::path::{Path, PathBuf};

pub use envoy::EnvoyConfig;
pub use haproxy::HaproxyConfig;
pub use nginx::NginxConfig;
pub use praxis::PraxisConfig;

// -----------------------------------------------------------------------------
// Config Resolution
// -----------------------------------------------------------------------------

/// Directory holding the built-in comparison proxy configs.
///
/// Resolved from `PRAXIS_BENCH_CONFIG_DIR` when set (the container image
/// points this at the bundled configs), otherwise the in-tree
/// `crates/praxis-bench/comparison/configs` relative to the repository root
/// for local `cargo run`.
pub(crate) fn config_dir() -> PathBuf {
    std::env::var_os("PRAXIS_BENCH_CONFIG_DIR").map_or_else(
        || PathBuf::from("crates/praxis-bench/comparison/configs"),
        PathBuf::from,
    )
}

// -----------------------------------------------------------------------------
// Proxy Config Trait
// -----------------------------------------------------------------------------

/// Configuration for a proxy server under test.
pub trait ProxyConfig: Send + Sync {
    /// Human-readable name (e.g. "praxis", "envoy").
    fn name(&self) -> &str;

    /// The address the proxy listens on (e.g. "127.0.0.1:8080").
    fn listen_address(&self) -> &str;

    /// Command and arguments to start the proxy.
    fn start_command(&self) -> (String, Vec<String>);

    /// Path to the proxy's configuration file.
    fn config_path(&self) -> &Path;

    /// Optional health-check URL. The runner will poll this
    /// before starting measurement.
    fn health_url(&self) -> Option<String> {
        None
    }

    /// Docker container name, if this proxy runs in Docker.
    fn container_name(&self) -> Option<&str> {
        None
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
#[expect(clippy::allow_attributes, reason = "blanket test suppressions")]
#[allow(clippy::unwrap_used, reason = "tests")]
mod tests {
    use super::*;

    /// A minimal [`ProxyConfig`] that relies on the trait's default
    /// `health_url` and `container_name`, which no built-in proxy exercises.
    struct DefaultedProxy;

    impl ProxyConfig for DefaultedProxy {
        fn name(&self) -> &str {
            "defaulted"
        }

        fn listen_address(&self) -> &str {
            "127.0.0.1:0"
        }

        fn start_command(&self) -> (String, Vec<String>) {
            ("noop".into(), Vec::new())
        }

        fn config_path(&self) -> &Path {
            Path::new("/dev/null")
        }
    }

    #[test]
    fn default_health_url_and_container_name_are_none() {
        let proxy = DefaultedProxy;
        assert_eq!(proxy.health_url(), None, "default health_url must be None");
        assert_eq!(proxy.container_name(), None, "default container_name must be None");
    }

    #[test]
    fn config_dir_is_non_empty() {
        assert!(
            !config_dir().as_os_str().is_empty(),
            "config_dir must resolve to a non-empty path"
        );
    }
}
