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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
#[expect(clippy::allow_attributes, reason = "blanket test suppressions")]
#[allow(clippy::unwrap_used, reason = "tests")]
mod tests {
    use clap::Parser as _;

    use super::*;

    /// The image argument appears as the final element of the docker command.
    fn image_arg(cfg: &dyn ProxyConfig) -> String {
        cfg.start_command().1.last().cloned().unwrap_or_default()
    }

    #[test]
    fn build_proxy_config_passes_envoy_image_override() {
        let args = Args::parse_from(["praxis-bench", "--envoy-image", "custom/envoy:tag"]);
        let cfg = build_proxy_config("envoy", &args, "praxis:img");
        assert_eq!(
            image_arg(cfg.as_ref()),
            "custom/envoy:tag",
            "the envoy image override must be threaded into the config"
        );
    }

    #[test]
    fn build_proxy_config_passes_nginx_image_override() {
        let args = Args::parse_from(["praxis-bench", "--nginx-image", "custom/nginx:tag"]);
        let cfg = build_proxy_config("nginx", &args, "praxis:img");
        assert_eq!(
            image_arg(cfg.as_ref()),
            "custom/nginx:tag",
            "the nginx image override must be threaded into the config"
        );
    }

    #[test]
    fn build_proxy_config_passes_haproxy_image_override() {
        let args = Args::parse_from(["praxis-bench", "--haproxy-image", "custom/haproxy:tag"]);
        let cfg = build_proxy_config("haproxy", &args, "praxis:img");
        assert_eq!(
            image_arg(cfg.as_ref()),
            "custom/haproxy:tag",
            "the haproxy image override must be threaded into the config"
        );
    }

    #[test]
    fn build_proxy_config_passes_praxis_image_under_test() {
        let args = Args::parse_from(["praxis-bench"]);
        let cfg = build_proxy_config("praxis", &args, "ghcr.io/example/praxis:testtag");
        assert_eq!(
            image_arg(cfg.as_ref()),
            "ghcr.io/example/praxis:testtag",
            "the praxis image under test must be threaded into the config"
        );
    }
}
