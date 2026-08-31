//! Report comparison and regression detection for `cargo xtask benchmark compare`.

use praxis_bench::result::{ComparativeResults, ScenarioResults};

use super::cli::CompareArgs;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum coefficient of variation allowed before skipping comparison.
const STABILITY_CV: f64 = 0.15;

// -----------------------------------------------------------------------------
// Comparison Computation
// -----------------------------------------------------------------------------

/// Compute comparisons of each non-praxis proxy against the
/// praxis baseline for matching scenarios.
pub(crate) fn compute_comparisons(
    results: &[ScenarioResults],
    proxy_names: &[String],
    threshold: f64,
) -> Vec<ComparativeResults> {
    let mut comparisons = Vec::new();
    if proxy_names.len() <= 1 {
        return comparisons;
    }

    for proxy in proxy_names.iter().skip(1) {
        for result in results.iter().filter(|r| r.proxy == *proxy) {
            if let Some(baseline) = results
                .iter()
                .find(|r| r.proxy == "praxis" && r.scenario == result.scenario)
            {
                comparisons.push(result.compare(baseline, threshold, None));
            }
        }
    }
    comparisons
}

// -----------------------------------------------------------------------------
// CLI Compare Command
// -----------------------------------------------------------------------------

/// Compare two benchmark reports and print a regression table.
///
/// Exits with code 1 if any scenario regressed beyond the
/// configured threshold.
pub(crate) fn run_compare(args: &CompareArgs) {
    let baseline = super::report::load_report(&args.baseline);
    let current = super::report::load_report(&args.current);

    print_comparison_header();
    let any_regressed = print_comparison_rows(&current, &baseline, args.threshold);

    if any_regressed {
        eprintln!("\nRegression detected!");
        std::process::exit(1);
    }
}

/// Print the comparison table header.
fn print_comparison_header() {
    println!(
        "{:<30} {:<10} {:>14} {:>14} {:>8}",
        "Scenario", "Proxy", "p99 Change %", "Thru Change %", "Status"
    );
    println!("{}", "-".repeat(80));
}

/// Print comparison rows for each praxis scenario; returns `true` if any regressed.
fn print_comparison_rows(
    current: &praxis_bench::report::BenchmarkReport,
    baseline: &praxis_bench::report::BenchmarkReport,
    threshold: f64,
) -> bool {
    let mut any_regressed = false;
    for cur_result in current.results.iter().filter(|r| r.proxy == "praxis") {
        let base = baseline
            .results
            .iter()
            .find(|r| r.proxy == "praxis" && r.scenario == cur_result.scenario);
        if let Some(base_result) = base {
            any_regressed |= print_comparison_row(cur_result, base_result, threshold);
        } else {
            println!(
                "{:<30} {:<10} {:>14} {:>14} {:>8}",
                cur_result.scenario, cur_result.proxy, "N/A", "N/A", "SKIP"
            );
        }
    }
    any_regressed
}

/// Print a single comparison row and return whether it regressed.
fn print_comparison_row(current: &ScenarioResults, baseline: &ScenarioResults, threshold: f64) -> bool {
    let cmp = current.compare(baseline, threshold, Some(STABILITY_CV));
    let status = if cmp.skipped {
        "SKIP"
    } else if cmp.regressed {
        "FAIL"
    } else {
        "PASS"
    };
    println!(
        "{:<30} {:<10} {:>13.1}% {:>13.1}% {:>8}",
        cmp.scenario,
        cmp.proxy,
        cmp.p99_latency_change * 100.0,
        cmp.throughput_change * 100.0,
        status,
    );
    cmp.regressed
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
#[expect(clippy::allow_attributes, reason = "blanket test suppressions")]
#[allow(clippy::too_many_lines, clippy::indexing_slicing, reason = "tests")]
mod tests {
    use std::collections::BTreeMap;

    use praxis_bench::{
        report::BenchmarkReport,
        result::{BenchmarkResult, Environment, ErrorMetrics, LatencyMetrics, ThroughputMetrics},
    };

    use super::*;

    /// A median [`BenchmarkResult`] fixed at the given p99/rps.
    fn median(p99: f64, rps: f64) -> BenchmarkResult {
        BenchmarkResult {
            commit: "c".into(),
            timestamp: "t".into(),
            scenario: "s".into(),
            proxy: "p".into(),
            tool: "vegeta".into(),
            environment: Environment {
                cpu: "x".into(),
                os: "linux".into(),
            },
            latency: LatencyMetrics {
                min: 0.0,
                max: 0.0,
                mean: 0.0,
                p50: 0.0,
                p90: 0.0,
                p95: 0.0,
                p99,
                p99_9: p99,
            },
            throughput: ThroughputMetrics {
                requests_per_sec: rps,
                bytes_per_sec: 0.0,
            },
            resource: None,
            errors: ErrorMetrics {
                non_2xx: Some(0),
                timeouts: 0,
                connect_failures: 0,
            },
            raw_report: None,
        }
    }

    /// [`ScenarioResults`] with an empty (hence stable) run set and a median.
    fn sr(scenario: &str, proxy: &str, p99: f64, rps: f64) -> ScenarioResults {
        ScenarioResults {
            scenario: scenario.into(),
            proxy: proxy.into(),
            runs: Vec::new(),
            median: Some(median(p99, rps)),
        }
    }

    fn report_with(results: Vec<ScenarioResults>) -> BenchmarkReport {
        BenchmarkReport {
            timestamp: "t".into(),
            commit: "c".into(),
            proxies: vec!["praxis".into()],
            settings: BTreeMap::new(),
            results,
            comparisons: Vec::new(),
        }
    }

    #[test]
    fn compute_comparisons_pairs_each_proxy_against_praxis() {
        // The non-praxis result is listed first so a wrong baseline lookup
        // (`||` / `!=` mutants) would grab envoy as its own baseline.
        let results = vec![
            sr("small", "envoy", 0.020, 5_000.0),
            sr("small", "praxis", 0.010, 10_000.0),
        ];
        let proxy_names = vec!["praxis".to_owned(), "envoy".to_owned()];
        let cmps = compute_comparisons(&results, &proxy_names, 0.05);
        assert_eq!(cmps.len(), 1, "one envoy scenario yields one comparison");
        assert_eq!(cmps[0].proxy, "envoy", "the comparison is for the non-praxis proxy");
        assert_eq!(cmps[0].scenario, "small", "the comparison matches the scenario");
        assert!(
            (cmps[0].p99_latency_change - 1.0).abs() < 1e-9,
            "envoy p99 is double the praxis baseline (100% change), got {}",
            cmps[0].p99_latency_change
        );
    }

    #[test]
    fn compute_comparisons_empty_for_single_proxy() {
        let results = vec![sr("small", "praxis", 0.010, 10_000.0)];
        let cmps = compute_comparisons(&results, &["praxis".to_owned()], 0.05);
        assert!(cmps.is_empty(), "a single proxy has nothing to compare against");
    }

    #[test]
    fn print_comparison_row_flags_regression() {
        let current = sr("s", "praxis", 0.020, 5_000.0);
        let baseline = sr("s", "praxis", 0.010, 10_000.0);
        assert!(
            print_comparison_row(&current, &baseline, 0.05),
            "doubled p99 with halved throughput must be a regression"
        );
    }

    #[test]
    fn print_comparison_row_passes_when_unchanged() {
        let current = sr("s", "praxis", 0.010, 10_000.0);
        let baseline = sr("s", "praxis", 0.010, 10_000.0);
        assert!(
            !print_comparison_row(&current, &baseline, 0.05),
            "identical results must not be a regression"
        );
    }

    #[test]
    fn print_comparison_rows_flags_any_regression() {
        // Scenario "a" improves, "b" regresses. The regression in "b" must be
        // reported: `|=` accumulates it, `&=` would drop it, and a mismatched
        // baseline lookup would compare "b" against the wrong row.
        let current = report_with(vec![
            sr("a", "praxis", 0.010, 10_000.0),
            sr("b", "praxis", 0.020, 5_000.0),
        ]);
        let baseline = report_with(vec![
            sr("a", "praxis", 0.020, 5_000.0),
            sr("b", "praxis", 0.010, 10_000.0),
        ]);
        assert!(
            print_comparison_rows(&current, &baseline, 0.05),
            "a regression in any scenario must be reported"
        );
    }

    #[test]
    fn print_comparison_rows_all_pass_returns_false() {
        let current = report_with(vec![sr("a", "praxis", 0.010, 10_000.0)]);
        let baseline = report_with(vec![sr("a", "praxis", 0.010, 10_000.0)]);
        assert!(
            !print_comparison_rows(&current, &baseline, 0.05),
            "no regressions must return false"
        );
    }
}
