use criterion::{black_box, criterion_group, criterion_main, Criterion};
use remote_trait_object_tests::{massive_no_export, massive_with_export};

pub fn no_export(c: &mut Criterion) {
    c.bench_function("no_export_100", |b| {
        b.iter(|| massive_no_export(black_box(100)))
    });
}

pub fn with_export(c: &mut Criterion) {
    c.bench_function("with_export_100", |b| {
        b.iter(|| massive_with_export(black_box(100)))
    });
}

criterion_group!(benches, no_export, with_export);
criterion_main!(benches);
