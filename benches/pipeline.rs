//! Benchmarks for the core pipeline hot paths: build+render for scatter,
//! histogram (stat_bin), and a proportion after_stat aggregate expression.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use ggplot_rs::prelude::*;

fn scatter_data(n: usize) -> Vec<(String, Vec<Value>)> {
    let x = (0..n).map(|i| Value::Float(i as f64 * 0.01)).collect();
    let y = (0..n)
        .map(|i| Value::Float((i as f64 * 0.01).sin()))
        .collect();
    let g = (0..n)
        .map(|i| Value::Str(["a", "b", "c"][i % 3].into()))
        .collect();
    vec![("x".into(), x), ("y".into(), y), ("g".into(), g)]
}

fn hist_data(n: usize) -> Vec<(String, Vec<Value>)> {
    let v = (0..n)
        .map(|i| Value::Float(((i * 7919 % 1000) as f64 / 100.0) - 5.0))
        .collect();
    vec![("v".into(), v)]
}

fn bench_scatter(c: &mut Criterion) {
    let mut g = c.benchmark_group("render_scatter_svg");
    for &n in &[1_000usize, 10_000] {
        let data = scatter_data(n);
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                let svg = GGPlot::new(data.clone())
                    .aes(Aes::new().x("x").y("y").color("g"))
                    .geom_point()
                    .render_svg()
                    .unwrap();
                black_box(svg.len())
            })
        });
    }
    g.finish();
}

fn bench_histogram(c: &mut Criterion) {
    let mut g = c.benchmark_group("build_histogram");
    for &n in &[1_000usize, 10_000] {
        let data = hist_data(n);
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                let built = GGPlot::new(data.clone())
                    .aes(Aes::new().x("v"))
                    .geom_histogram()
                    .build();
                black_box(built.layers.len())
            })
        });
    }
    g.finish();
}

fn bench_aggregate_expr(c: &mut Criterion) {
    // Exercises the after_stat aggregate path (count / sum(count)) — the fold
    // pass keeps this O(n).
    let mut g = c.benchmark_group("after_stat_proportion");
    for &n in &[1_000usize, 10_000] {
        let data = hist_data(n);
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                let built = GGPlot::new(data.clone())
                    .aes(Aes::new().x("v").after_stat_y("count / sum(count)"))
                    .geom_histogram()
                    .build();
                black_box(built.layers.len())
            })
        });
    }
    g.finish();
}

criterion_group!(
    benches,
    bench_scatter,
    bench_histogram,
    bench_aggregate_expr
);
criterion_main!(benches);
