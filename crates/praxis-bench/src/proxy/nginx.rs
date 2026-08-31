//! Built-in proxy configuration for NGINX.

use std::path::PathBuf;

use super::ProxyConfig;

// -----------------------------------------------------------------------------
// NginxConfig
// -----------------------------------------------------------------------------

/// Built-in [`ProxyConfig`] for NGINX via Docker.
#[derive(Debug)]
pub struct NginxConfig {
    /// Listen address on the host (e.g. "127.0.0.1:8080").
    pub address: String,

    /// Path to the NGINX config file.
    pub config: PathBuf,

    /// Docker container name.
    pub container_name: String,

    /// Optional Docker image override.
    pub image: Option<String>,
}

impl Default for NginxConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1:18092".into(),
            config: super::config_dir().join("nginx.conf"),
            container_name: "praxis-bench-nginx".into(),
            image: None,
        }
    }
}

impl ProxyConfig for NginxConfig {
    fn name(&self) -> &str {
        "nginx"
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
                format!("{}:/etc/nginx/nginx.conf:ro", config_abs.display()),
                self.image.as_deref().unwrap_or("nginx:alpine").to_owned(),
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
    fn nginx_config_defaults() {
        let config = NginxConfig::default();

        assert_eq!(config.name(), "nginx");
        assert_eq!(config.listen_address(), "127.0.0.1:18092");
        assert_eq!(config.container_name(), Some("praxis-bench-nginx"));
        assert_eq!(config.health_url(), None, "nginx has no built-in health URL");
    }

    #[test]
    fn nginx_start_command_runs_docker_with_defaults() {
        let config = NginxConfig::default();
        let (cmd, args) = config.start_command();

        assert_eq!(cmd, "docker", "nginx runs via docker");
        assert!(args.contains(&"run".to_owned()), "must be a docker run");
        assert!(args.contains(&"--name".to_owned()), "must name the container");
        assert!(
            args.contains(&"praxis-bench-nginx".to_owned()),
            "must pass the container name"
        );
        assert!(args.contains(&"--cpus=4.0".to_owned()), "must apply the CPU limit");
        assert!(
            args.contains(&"nginx:alpine".to_owned()),
            "must fall back to the default nginx image"
        );
    }

    #[test]
    fn nginx_start_command_uses_image_override() {
        let config = NginxConfig {
            image: Some("myregistry/nginx:test".into()),
            ..Default::default()
        };
        let (_cmd, args) = config.start_command();
        assert!(
            args.contains(&"myregistry/nginx:test".to_owned()),
            "an image override must appear in the command"
        );
    }
}
