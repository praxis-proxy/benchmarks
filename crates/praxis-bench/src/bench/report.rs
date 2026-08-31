//! Benchmark report serialization and deserialization.

use praxis_bench::report::BenchmarkReport;

// -----------------------------------------------------------------------------
// Load
// -----------------------------------------------------------------------------

/// Load a [`BenchmarkReport`] from a file, detecting format from the extension.
///
/// [`BenchmarkReport`]: praxis_bench::report::BenchmarkReport
pub(crate) fn load_report(path: &str) -> BenchmarkReport {
    let content = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("failed to read {path}: {e}");
        std::process::exit(1);
    });

    if path.ends_with(".json") {
        serde_json::from_str(&content).unwrap_or_else(|e| {
            eprintln!("failed to parse JSON: {e}");
            std::process::exit(1);
        })
    } else {
        serde_yaml::from_str(&content).unwrap_or_else(|e| {
            eprintln!("failed to parse YAML: {e}");
            std::process::exit(1);
        })
    }
}

// -----------------------------------------------------------------------------
// Write
// -----------------------------------------------------------------------------

/// Serialize and write the report to `path` in the given format (`yaml` or `json`).
pub(crate) fn write_report(report: &BenchmarkReport, path: &str, format: &str) {
    let content = match format {
        "json" => serde_json::to_string_pretty(report).expect("failed to serialize report to JSON"),
        _ => serde_yaml::to_string(report).expect("failed to serialize report to YAML"),
    };
    std::fs::write(path, content).unwrap_or_else(|e| {
        eprintln!("failed to write report: {e}");
        std::process::exit(1);
    });
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn empty_report() -> BenchmarkReport {
        BenchmarkReport {
            timestamp: "2026-01-01T00:00:00Z".into(),
            commit: "abc123".into(),
            proxies: vec!["praxis".into()],
            settings: BTreeMap::new(),
            results: Vec::new(),
            comparisons: Vec::new(),
        }
    }

    #[test]
    fn write_report_json_emits_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.json");
        write_report(&empty_report(), path.to_str().unwrap(), "json");
        let content = std::fs::read_to_string(&path).expect("report file must be written");
        assert!(
            content.trim_start().starts_with('{'),
            "json format must emit a JSON object"
        );
        assert!(
            content.contains("\"commit\""),
            "json must quote field names, got: {content}"
        );
    }

    #[test]
    fn write_report_yaml_emits_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.yaml");
        write_report(&empty_report(), path.to_str().unwrap(), "yaml");
        let content = std::fs::read_to_string(&path).expect("report file must be written");
        assert!(!content.trim_start().starts_with('{'), "yaml must not be a JSON object");
        assert!(
            content.contains("commit:"),
            "yaml must use mapping keys, got: {content}"
        );
    }
}
