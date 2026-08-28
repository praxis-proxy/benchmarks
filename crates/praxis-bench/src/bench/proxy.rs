//! Proxy configuration builders for benchmark runs.
//!
//! The proxy under test is treated as an opaque container image supplied
//! by the operator (`--image`), so any Praxis-compatible build can be
//! benchmarked without this tool depending on its source.

use praxis_bench::proxy::{EnvoyConfig, HaproxyConfig, NginxConfig, PraxisConfig, ProxyConfig};

use super::cli::Args;

// -----------------------------------------------------------------------------
// Proxy Config Factory
// -----------------------------------------------------------------------------

/// Build a boxed [`ProxyConfig`] for the named proxy.
///
/// All proxies run containerized with identical resource constraints.
///
/// [`ProxyConfig`]: praxis_bench::proxy::ProxyConfig
pub(crate) fn build_proxy_config(name: &str, args: &Args, praxis_image: &str) -> Box<dyn ProxyConfig> {
    match name {
        "praxis" => Box::new(PraxisConfig::new(praxis_image.to_owned())),
        "envoy" => Box::new(EnvoyConfig {
            image: Some(args.envoy_image.clone()),
            ..Default::default()
        }),
        "nginx" => Box::new(NginxConfig {
            image: Some(args.nginx_image.clone()),
            ..Default::default()
        }),
        "haproxy" => Box::new(HaproxyConfig {
            image: Some(args.haproxy_image.clone()),
            ..Default::default()
        }),
        other => {
            tracing::error!(proxy = other, "unknown proxy");
            std::process::exit(1);
        },
    }
}
