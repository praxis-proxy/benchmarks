//! `cargo xtask benchmark visualize` SVG chart generator.

use clap::Parser;
use praxis_bench::report::BenchmarkReport;

// -----------------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------------

/// SVG dimensions.
const SVG_WIDTH: u32 = 1400;

/// Height per panel.
const PANEL_HEIGHT: u32 = 420;

/// Left margin for y-axis labels.
const LEFT_MARGIN: i32 = 90;

/// Top margin per panel (for title).
const TOP_MARGIN: i32 = 35;

/// Bottom margin per panel (for x-axis labels).
const BOTTOM_MARGIN: i32 = 80;

/// All charts to render.
#[expect(clippy::cast_precision_loss, reason = "chart values")]
const CHARTS: &[ChartDef] = &[
    ChartDef {
        suffix: "p99-latency",
        title: "p99 Latency (ms)  \u{2193} lower is better",
        y_label: "ms",
        extract: |m| m.latency.p99 * 1000.0,
    },
    ChartDef {
        suffix: "throughput",
        title: "Throughput (req/s)  \u{2191} higher is better",
        y_label: "req/s",
        extract: |m| m.throughput.requests_per_sec,
    },
    ChartDef {
        suffix: "min-latency",
        title: "Min Latency (ms)  \u{2193} lower is better",
        y_label: "ms",
        extract: |m| m.latency.min * 1000.0,
    },
    ChartDef {
        suffix: "mean-latency",
        title: "Mean Latency (ms)  \u{2193} lower is better",
        y_label: "ms",
        extract: |m| m.latency.mean * 1000.0,
    },
    ChartDef {
        suffix: "max-latency",
        title: "Max Latency (ms)  \u{2193} lower is better",
        y_label: "ms",
        extract: |m| m.latency.max * 1000.0,
    },
    ChartDef {
        suffix: "data-throughput",
        title: "Data Throughput (MB/s)  \u{2191} higher is better",
        y_label: "MB/s",
        extract: |m| m.throughput.bytes_per_sec / 1_000_000.0,
    },
    ChartDef {
        suffix: "cpu-avg",
        title: "Average CPU Utilization (%)  \u{2193} lower is better",
        y_label: "%",
        extract: |m| m.resource.as_ref().map_or(0.0, |r| r.cpu_percent_avg),
    },
    ChartDef {
        suffix: "memory-peak",
        title: "Peak Memory RSS (MiB)  \u{2193} lower is better",
        y_label: "MiB",
        extract: |m| {
            m.resource
                .as_ref()
                .map_or(0.0, |r| r.memory_rss_bytes_peak as f64 / 1_048_576.0)
        },
    },
];

// -----------------------------------------------------------------------------
// CLI Arguments
// -----------------------------------------------------------------------------

/// CLI arguments for `cargo xtask benchmark visualize`.
#[derive(Parser)]
pub(crate) struct Args {
    /// Path to the benchmark report file (YAML or JSON).
    pub file: String,

    /// Output directory for SVG files. Defaults to
    /// `target/criterion`.
    #[arg(long)]
    pub output: Option<String>,
}

// -----------------------------------------------------------------------------
// Entry Point
// -----------------------------------------------------------------------------

/// Generate SVG charts from a benchmark report.
///
/// Produces one SVG per metric in the output directory.
pub(crate) fn run(args: &Args) {
    let report = super::report::load_report(&args.file);

    let stem = std::path::Path::new(&args.file)
        .file_stem()
        .map_or("benchmark", |s| s.to_str().unwrap_or("benchmark"));

    let dir = args.output.clone().unwrap_or_else(|| "target/criterion".into());

    std::fs::create_dir_all(&dir).ok();

    render_charts(&report, stem, &dir);
}

// -----------------------------------------------------------------------------
// Data Extraction
// -----------------------------------------------------------------------------

/// Unique scenario names from a report (first-seen order).
fn unique_scenarios(report: &BenchmarkReport) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();

    report
        .results
        .iter()
        .filter(|&r| seen.insert(r.scenario.clone()))
        .map(|r| r.scenario.clone())
        .collect()
}

/// Extract a per-proxy, per-scenario metric matrix.
fn extract_matrix<F>(report: &BenchmarkReport, scenarios: &[String], metric: F) -> Vec<Vec<f64>>
where
    F: Fn(&praxis_bench::result::BenchmarkResult) -> f64,
{
    report
        .proxies
        .iter()
        .map(|proxy| {
            scenarios
                .iter()
                .map(|scenario| {
                    report
                        .results
                        .iter()
                        .find(|r| r.proxy == *proxy && r.scenario == *scenario)
                        .and_then(|r| r.median.as_ref())
                        .map_or(0.0, &metric)
                })
                .collect()
        })
        .collect()
}

// -----------------------------------------------------------------------------
// SVG Rendering
// -----------------------------------------------------------------------------

/// Chart definition: metric name, file suffix, title,
/// y-axis label, and extraction function.
struct ChartDef {
    /// File name suffix (e.g. "p99-latency").
    suffix: &'static str,

    /// Chart title.
    title: &'static str,

    /// Y-axis label.
    y_label: &'static str,

    /// Metric extractor.
    extract: fn(&praxis_bench::result::BenchmarkResult) -> f64,
}

/// Map a proxy name to its chart bar color.
fn proxy_color(name: &str) -> plotters::style::RGBColor {
    match name {
        "praxis" => plotters::style::RGBColor(76, 175, 80),
        "envoy" => plotters::style::RGBColor(33, 150, 243),
        "nginx" => plotters::style::RGBColor(244, 67, 54),
        "haproxy" => plotters::style::RGBColor(156, 39, 176),
        _ => plotters::style::RGBColor(158, 158, 158),
    }
}

/// Render one SVG per metric into the output directory.
fn render_charts(report: &BenchmarkReport, stem: &str, dir: &str) {
    use plotters::prelude::{IntoDrawingArea as _, SVGBackend, WHITE};

    let scenarios = unique_scenarios(report);

    if scenarios.is_empty() {
        eprintln!("no scenario data to visualize");
        return;
    }

    for chart in CHARTS {
        let path = format!("{dir}/{stem}-{suffix}.svg", suffix = chart.suffix);

        let data = extract_matrix(report, &scenarios, chart.extract);

        let root = SVGBackend::new(&path, (SVG_WIDTH, PANEL_HEIGHT)).into_drawing_area();

        root.fill(&WHITE).unwrap();

        render_panel(&root, chart.title, chart.y_label, &report.proxies, &scenarios, &data);

        root.present().unwrap_or_else(|e| {
            eprintln!("failed to write SVG {path}: {e}");
        });

        println!("  {path}");
    }
}

/// Type alias for the 2D chart context produced by [`render_panel`].
///
/// Concrete over [`plotters::prelude::SVGBackend`] to avoid generic
/// lifetime invariance issues in plotters' `DrawingArea`.
type PanelChart<'a, 'b> = plotters::prelude::ChartContext<
    'a,
    plotters::prelude::SVGBackend<'b>,
    plotters::prelude::Cartesian2d<plotters::coord::types::RangedCoordf64, plotters::coord::types::RangedCoordf64>,
>;

/// Render a single grouped bar chart panel.
#[expect(clippy::cast_precision_loss, clippy::too_many_arguments, reason = "chart rendering")]
fn render_panel(
    area: &plotters::prelude::DrawingArea<plotters::prelude::SVGBackend<'_>, plotters::coord::Shift>,
    title: &str,
    y_label: &str,
    proxies: &[String],
    scenarios: &[String],
    bars: &[Vec<f64>],
) {
    use plotters::{prelude::ChartBuilder, style::IntoFont as _};

    let n_proxies = proxies.len();
    let n_scenarios = scenarios.len();
    let group_width = n_proxies as f64 + 1.0;
    let x_max = n_scenarios as f64 * group_width;

    let max_val = bars.iter().flat_map(|v| v.iter()).copied().fold(0.0_f64, f64::max) * 1.15;
    let max_val = if max_val == 0.0 { 1.0 } else { max_val };

    let mut chart = ChartBuilder::on(area)
        .caption(title, ("sans-serif", 18).into_font())
        .margin_top(TOP_MARGIN as u32)
        .margin_left(10)
        .margin_right(10)
        .x_label_area_size(BOTTOM_MARGIN as u32)
        .y_label_area_size(LEFT_MARGIN as u32)
        .build_cartesian_2d(0.0..x_max, 0.0..max_val)
        .unwrap();

    configure_mesh(&mut chart, y_label);
    draw_bars(&mut chart, proxies, n_scenarios, group_width, bars);
    draw_scenario_labels(&mut chart, scenarios, n_proxies, group_width, max_val);
    draw_legend(&mut chart);
}

/// Configure the y-axis mesh, hiding the x-axis grid.
fn configure_mesh(chart: &mut PanelChart<'_, '_>, y_label: &str) {
    use plotters::style::IntoFont as _;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .disable_x_axis()
        .y_desc(y_label)
        .y_label_style(("sans-serif", 12).into_font())
        .draw()
        .unwrap();
}

/// Draw grouped bars for each proxy across all scenarios.
#[expect(clippy::cast_precision_loss, clippy::indexing_slicing, reason = "chart coordinates")]
fn draw_bars(
    chart: &mut PanelChart<'_, '_>,
    proxies: &[String],
    n_scenarios: usize,
    group_width: f64,
    bars: &[Vec<f64>],
) {
    use plotters::{prelude::Rectangle, style::Color as _};

    for (pi, proxy) in proxies.iter().enumerate() {
        let color = proxy_color(proxy);

        let rects: Vec<_> = (0..n_scenarios)
            .map(|si| {
                let x0 = si as f64 * group_width + pi as f64 + 0.5;
                let x1 = x0 + 0.8;
                let y = bars[pi][si];
                Rectangle::new([(x0, 0.0), (x1, y)], color.filled())
            })
            .collect();

        chart
            .draw_series(rects)
            .unwrap()
            .label(proxy.as_str())
            .legend(move |(x, y)| Rectangle::new([(x, y - 5), (x + 15, y + 5)], color.filled()));
    }
}

/// Draw scenario name labels along the x-axis.
#[expect(clippy::cast_precision_loss, reason = "chart coordinates")]
fn draw_scenario_labels(
    chart: &mut PanelChart<'_, '_>,
    scenarios: &[String],
    n_proxies: usize,
    group_width: f64,
    max_val: f64,
) {
    use plotters::{
        prelude::BLACK,
        style::{IntoFont as _, TextStyle},
    };

    let label_style = TextStyle::from(("sans-serif", 10).into_font()).color(&BLACK);

    for (si, name) in scenarios.iter().enumerate() {
        let center_x = si as f64 * group_width + n_proxies as f64 / 2.0 + 0.5;
        let short = shorten_scenario(name);

        chart
            .draw_series(std::iter::once(plotters::element::Text::new(
                short,
                (center_x, -max_val * 0.02),
                label_style.clone(),
            )))
            .unwrap();
    }
}

/// Draw the series legend in the upper-right corner.
fn draw_legend<'a>(chart: &mut PanelChart<'a, 'a>) {
    use plotters::{
        prelude::{BLACK, SeriesLabelPosition, WHITE},
        style::{Color as _, IntoFont as _},
    };

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperRight)
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .label_font(("sans-serif", 12).into_font())
        .draw()
        .unwrap();
}

/// Shorten scenario names for chart labels.
fn shorten_scenario(name: &str) -> String {
    match name {
        "high-concurrency-small-requests" => "small-req".into(),
        "large-payloads" => "large".into(),
        "large-payloads-high-concurrency" => "large-hc".into(),
        "high-connection-count" => "high-conn".into(),
        "tcp-throughput" => "tcp-thru".into(),
        "tcp-connection-rate" => "tcp-conn".into(),
        other => other.into(),
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
#[expect(clippy::allow_attributes, reason = "blanket test suppressions")]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    reason = "tests"
)]
mod tests {
    use praxis_bench::result::{
        BenchmarkResult, Environment, ErrorMetrics, LatencyMetrics, ResourceMetrics, ScenarioResults, ThroughputMetrics,
    };

    use super::*;

    /// Build a [`BenchmarkResult`] with the given per-metric values.
    fn result(scenario: &str, proxy: &str, latency: LatencyMetrics, throughput: ThroughputMetrics) -> BenchmarkResult {
        BenchmarkResult {
            commit: "c".into(),
            timestamp: "t".into(),
            scenario: scenario.into(),
            proxy: proxy.into(),
            tool: "vegeta".into(),
            environment: Environment {
                cpu: "x".into(),
                os: "linux".into(),
            },
            latency,
            throughput,
            resource: None,
            errors: ErrorMetrics {
                non_2xx: Some(0),
                timeouts: 0,
                connect_failures: 0,
            },
            raw_report: None,
        }
    }

    /// A [`ScenarioResults`] whose median has the given p99 latency.
    fn scenario_with_p99(scenario: &str, proxy: &str, p99: f64) -> ScenarioResults {
        let mut latency = LatencyMetrics::zeroed();
        latency.p99 = p99;
        ScenarioResults {
            scenario: scenario.into(),
            proxy: proxy.into(),
            runs: Vec::new(),
            median: Some(result(
                scenario,
                proxy,
                latency,
                ThroughputMetrics {
                    requests_per_sec: 0.0,
                    bytes_per_sec: 0.0,
                },
            )),
        }
    }

    #[test]
    fn chart_extractors_scale_each_metric() {
        // Distinctive inputs so a `+`, `/`, `*` or `%` mutant in any extractor
        // closure produces a value that fails its assertion.
        let latency = LatencyMetrics {
            min: 0.001,
            max: 0.1,
            mean: 0.01,
            p50: 0.02,
            p90: 0.03,
            p95: 0.04,
            p99: 0.05,
            p99_9: 0.06,
        };
        let mut r = result(
            "s",
            "praxis",
            latency,
            ThroughputMetrics {
                requests_per_sec: 1234.0,
                bytes_per_sec: 2_000_000.0,
            },
        );
        r.resource = Some(ResourceMetrics {
            cpu_percent_avg: 42.0,
            cpu_percent_peak: 99.0,
            memory_rss_bytes_avg: 1_048_576,
            memory_rss_bytes_peak: 3_145_728,
        });

        for chart in CHARTS {
            let got = (chart.extract)(&r);
            let expected = match chart.suffix {
                "p99-latency" => 50.0,    // 0.05 * 1000
                "throughput" => 1234.0,   // requests_per_sec
                "min-latency" => 1.0,     // 0.001 * 1000
                "mean-latency" => 10.0,   // 0.01 * 1000
                "max-latency" => 100.0,   // 0.1 * 1000
                "data-throughput" => 2.0, // 2_000_000 / 1_000_000
                "cpu-avg" => 42.0,        // cpu_percent_avg
                "memory-peak" => 3.0,     // 3_145_728 / 1_048_576
                other => panic!("unexpected chart suffix: {other}"),
            };
            assert!(
                (got - expected).abs() < 1e-9,
                "chart {} should extract {expected}, got {got}",
                chart.suffix
            );
        }
    }

    #[test]
    fn unique_scenarios_dedupes_in_first_seen_order() {
        let report = BenchmarkReport {
            timestamp: "t".into(),
            commit: "c".into(),
            proxies: vec!["praxis".into()],
            settings: std::collections::BTreeMap::new(),
            results: vec![
                scenario_with_p99("x", "praxis", 0.0),
                scenario_with_p99("y", "praxis", 0.0),
                scenario_with_p99("x", "envoy", 0.0),
            ],
            comparisons: Vec::new(),
        };
        assert_eq!(
            unique_scenarios(&report),
            vec!["x".to_owned(), "y".to_owned()],
            "scenarios must be de-duplicated in first-seen order"
        );
    }

    #[test]
    fn extract_matrix_indexes_by_proxy_then_scenario() {
        // 2x2 matrix: a mis-wired `find` (`||`/`!=` mutants) or a placeholder
        // return would break exact per-cell values.
        let report = BenchmarkReport {
            timestamp: "t".into(),
            commit: "c".into(),
            proxies: vec!["praxis".into(), "envoy".into()],
            settings: std::collections::BTreeMap::new(),
            results: vec![
                scenario_with_p99("a", "praxis", 1.0),
                scenario_with_p99("b", "praxis", 2.0),
                scenario_with_p99("a", "envoy", 3.0),
                scenario_with_p99("b", "envoy", 4.0),
            ],
            comparisons: Vec::new(),
        };
        let scenarios = vec!["a".to_owned(), "b".to_owned()];
        let matrix = extract_matrix(&report, &scenarios, |r| r.latency.p99);
        assert_eq!(
            matrix,
            vec![vec![1.0, 2.0], vec![3.0, 4.0]],
            "cells must map [proxy][scenario]"
        );
    }

    #[test]
    fn extract_matrix_missing_cell_is_zero() {
        let report = BenchmarkReport {
            timestamp: "t".into(),
            commit: "c".into(),
            proxies: vec!["praxis".into()],
            settings: std::collections::BTreeMap::new(),
            results: vec![scenario_with_p99("a", "praxis", 1.0)],
            comparisons: Vec::new(),
        };
        let scenarios = vec!["a".to_owned(), "missing".to_owned()];
        let matrix = extract_matrix(&report, &scenarios, |r| r.latency.p99);
        assert_eq!(matrix, vec![vec![1.0, 0.0]], "a missing scenario cell defaults to 0.0");
    }

    #[test]
    fn proxy_color_maps_known_proxies() {
        let rgb = |name| {
            let c = proxy_color(name);
            (c.0, c.1, c.2)
        };
        assert_eq!(rgb("praxis"), (76, 175, 80), "praxis colour");
        assert_eq!(rgb("envoy"), (33, 150, 243), "envoy colour");
        assert_eq!(rgb("nginx"), (244, 67, 54), "nginx colour");
        assert_eq!(rgb("haproxy"), (156, 39, 176), "haproxy colour");
        assert_eq!(rgb("unknown"), (158, 158, 158), "unknown proxies fall back to grey");
    }

    #[test]
    fn shorten_scenario_maps_known_and_passes_through() {
        assert_eq!(shorten_scenario("high-concurrency-small-requests"), "small-req");
        assert_eq!(shorten_scenario("tcp-throughput"), "tcp-thru");
        assert_eq!(
            shorten_scenario("something-else"),
            "something-else",
            "unknown names pass through"
        );
    }

    #[test]
    fn run_writes_one_svg_per_chart() {
        let dir = tempfile::tempdir().unwrap();
        let report = BenchmarkReport {
            timestamp: "t".into(),
            commit: "c".into(),
            proxies: vec!["praxis".into(), "envoy".into()],
            settings: std::collections::BTreeMap::new(),
            results: vec![
                scenario_with_p99("high-concurrency-small-requests", "praxis", 0.01),
                scenario_with_p99("high-concurrency-small-requests", "envoy", 0.02),
            ],
            comparisons: Vec::new(),
        };
        let report_path = dir.path().join("bench.yaml");
        std::fs::write(&report_path, serde_yaml::to_string(&report).unwrap()).unwrap();
        let out_dir = dir.path().join("charts");

        let args = Args {
            file: report_path.to_str().unwrap().to_owned(),
            output: Some(out_dir.to_str().unwrap().to_owned()),
        };
        run(&args);

        let svgs = std::fs::read_dir(&out_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "svg"))
            .count();
        assert_eq!(svgs, CHARTS.len(), "run must write one SVG per chart");
    }
}
