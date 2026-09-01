use criterion::{criterion_group, criterion_main, Criterion};
use shutdown_kit::ShutdownConfig;
use shutdown_kit::ShutdownGuard;
use std::time::Duration;

fn bench_config_defaults(c: &mut Criterion) {
    c.bench_function("config_defaults", |b| {
        b.iter(|| {
            let config = ShutdownConfig::defaults();
            std::hint::black_box(config);
        });
    });
}

fn bench_config_default_trait(c: &mut Criterion) {
    c.bench_function("config_default_trait", |b| {
        b.iter(|| {
            let config = ShutdownConfig::default();
            std::hint::black_box(config);
        });
    });
}

fn bench_config_builder(c: &mut Criterion) {
    c.bench_function("config_builder", |b| {
        b.iter(|| {
            let config = ShutdownConfig::builder()
                .drain_timeout(Duration::from_secs(60))
                .shutdown_timeout(Duration::from_secs(20))
                .build();
            std::hint::black_box(config);
        });
    });
}

fn bench_config_builder_defaults(c: &mut Criterion) {
    c.bench_function("config_builder_defaults", |b| {
        b.iter(|| {
            let config = ShutdownConfig::builder().build();
            std::hint::black_box(config);
        });
    });
}

fn bench_guard_new(c: &mut Criterion) {
    c.bench_function("guard_new", |b| {
        b.iter(|| {
            let guard = ShutdownGuard::new();
            std::hint::black_box(guard);
        });
    });
}

fn bench_guard_clone(c: &mut Criterion) {
    c.bench_function("guard_clone", |b| {
        let guard = ShutdownGuard::new();
        b.iter(|| {
            let cloned = guard.clone();
            std::hint::black_box(cloned);
        });
    });
}

fn bench_guard_is_shutdown(c: &mut Criterion) {
    let guard = ShutdownGuard::new();
    c.bench_function("guard_is_shutdown", |b| {
        b.iter(|| {
            let is_shutdown = guard.is_shutdown();
            std::hint::black_box(is_shutdown);
        });
    });
}

fn bench_guard_shutdown(c: &mut Criterion) {
    c.bench_function("guard_shutdown", |b| {
        b.iter(|| {
            let guard = ShutdownGuard::new();
            let _clone = guard.clone();
            guard.shutdown();
        });
    });
}

criterion_group!(
    benches,
    bench_config_defaults,
    bench_config_default_trait,
    bench_config_builder,
    bench_config_builder_defaults,
    bench_guard_new,
    bench_guard_clone,
    bench_guard_is_shutdown,
    bench_guard_shutdown,
);
criterion_main!(benches);
