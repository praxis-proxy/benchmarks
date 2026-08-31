//! Built-in proxy configuration for `HAProxy`.

use std::path::PathBuf;

use super::ProxyConfig;

// -----------------------------------------------------------------------------
// HaproxyConfig
// -----------------------------------------------------------------------------

/// Built-in [`ProxyConfig`] for `HAProxy` via Docker.
#[derive(Debug)]
pub struct HaproxyConfig {
    /// Listen address on the host (e.g. "127.0.0.1:8080").
    pub address: String,

    /// Path to the `HAProxy` config file.
    pub config: PathBuf,

    /// Docker container name.
    pub container_name: String,

    /// Optional Docker image override.
    pub image: Option<String>,
}

impl Default for HaproxyConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1:18093".into(),
            config: super::config_dir().join("haproxy.cfg"),
            container_name: "praxis-bench-haproxy".into(),
            image: None,
        }
    }
}

impl ProxyConfig for HaproxyConfig {
    fn name(&self) -> &str {
        "haproxy"
    }

    fn listen_address(&self) -> &str {
        &self.address
    }

    fn start_command(&self) -> (String, Vec<String>) {
        let config_abs = std::fs::canonicalize(&self.config).unwrap_or_else(|_| self.config.clone());

        (
            "docker".into(),
            vec![
                "run".into(),
                "--rm".into(),
                "--name".into(),
                self.container_name.clone(),
                "--network".into(),
                "host".into(),
                "--cpus=4.0".into(),
                "--memory=2g".into(),
                "-v".into(),
                format!("{}:/usr/local/etc/haproxy/haproxy.cfg:ro", config_abs.display()),
                self.image.as_deref().unwrap_or("haproxy:latest").to_owned(),
            ],
        )
    }

    fn config_path(&self) -> &std::path::Path {
        &self.config
    }

    fn container_name(&self) -> Option<&str> {
        Some(&self.container_name)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
#[expect(clippy::allow_attributes, reason = "blanket test suppressions")]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, reason = "tests")]
mod tests {
    use super::*;

    #[test]
    fn haproxy_config_defaults() {
        let config = HaproxyConfig::default();

        assert_eq!(config.name(), "haproxy");
        assert_eq!(config.listen_address(), "127.0.0.1:18093");
        assert_eq!(config.container_name(), Some("praxis-bench-haproxy"));
        assert_eq!(config.health_url(), None, "haproxy has no built-in health URL");
    }

    #[test]
    fn haproxy_start_command_runs_docker_with_defaults() {
        let config = HaproxyConfig::default();
        let (cmd, args) = config.start_command();

        assert_eq!(cmd, "docker", "haproxy runs via docker");
        assert!(args.contains(&"run".to_owned()), "must be a docker run");
        assert!(args.contains(&"--name".to_owned()), "must name the container");
        assert!(
            args.contains(&"praxis-bench-haproxy".to_owned()),
            "must pass the container name"
        );
        assert!(args.contains(&"--cpus=4.0".to_owned()), "must apply the CPU limit");
        assert!(
            args.contains(&"haproxy:latest".to_owned()),
            "must fall back to the default haproxy image"
        );
    }

    #[test]
    fn haproxy_start_command_uses_image_override() {
        let config = HaproxyConfig {
            image: Some("myregistry/haproxy:test".into()),
            ..Default::default()
        };
        let (_cmd, args) = config.start_command();
        assert!(
            args.contains(&"myregistry/haproxy:test".to_owned()),
            "an image override must appear in the command"
        );
    }
}
