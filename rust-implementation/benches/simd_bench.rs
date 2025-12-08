//! Benchmarks de operações SIMD e vetoriais.
//!
//! Testa performance de:
//! - Similaridade cosseno (scalar vs AVX2)
//! - Produto escalar (dot product)
//! - Busca de vetores similares
//! - Deduplicação de queries
//!
//! Executar: `cargo bench --bench simd_bench`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use deep_research::performance::simd::{
    cosine_similarity, cosine_similarity_scalar, dedup_queries, dot_product, find_similar, l2_norm,
    normalize,
};
use rand::Rng;

/// Gera vetor de embeddings aleatórios
fn generate_random_embedding(dim: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect()
}

/// Gera múltiplos embeddings
fn generate_embeddings(count: usize, dim: usize) -> Vec<Vec<f32>> {
    (0..count).map(|_| generate_random_embedding(dim)).collect()
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Similaridade Cosseno
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_cosine_similarity(c: &mut Criterion) {
    let mut group = c.benchmark_group("cosine_similarity");

    // Testar diferentes tamanhos de vetores
    for size in [8, 64, 256, 768, 1536, 3072].iter() {
        let a = generate_random_embedding(*size);
        let b = generate_random_embedding(*size);

        group.throughput(Throughput::Elements(*size as u64));

        // Versão scalar (baseline)
        group.bench_with_input(BenchmarkId::new("scalar", size), size, |bencher, _| {
            bencher.iter(|| black_box(cosine_similarity_scalar(&a, &b)))
        });

        // Versão auto-detect (SIMD quando disponível)
        group.bench_with_input(BenchmarkId::new("auto", size), size, |bencher, _| {
            bencher.iter(|| black_box(cosine_similarity(&a, &b)))
        });
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Dot Product
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_dot_product(c: &mut Criterion) {
    let mut group = c.benchmark_group("dot_product");

    for size in [768, 1536, 3072].iter() {
        let a = generate_random_embedding(*size);
        let b = generate_random_embedding(*size);

        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("optimized", size), size, |bencher, _| {
            bencher.iter(|| black_box(dot_product(&a, &b)))
        });
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Normalização
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_normalize(c: &mut Criterion) {
    let mut group = c.benchmark_group("normalize");

    for size in [768, 1536].iter() {
        group.bench_with_input(BenchmarkId::new("l2_norm", size), size, |bencher, &size| {
            let v = generate_random_embedding(size);
            bencher.iter(|| black_box(l2_norm(&v)))
        });

        group.bench_with_input(
            BenchmarkId::new("normalize", size),
            size,
            |bencher, &size| {
                bencher.iter(|| {
                    let mut v = generate_random_embedding(size);
                    normalize(&mut v);
                    black_box(v)
                })
            },
        );
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Busca de Similares (Paralelismo)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_find_similar(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_similar");
    group.sample_size(50); // Menos samples para benchmarks mais lentos

    let dim = 768;
    let query = generate_random_embedding(dim);

    // Testar diferentes quantidades de embeddings existentes
    for count in [100, 500, 1000, 5000].iter() {
        let existing = generate_embeddings(*count, dim);

        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(BenchmarkId::new("parallel", count), count, |bencher, _| {
            bencher.iter(|| black_box(find_similar(&query, &existing, 0.8)))
        });
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Deduplicação de Queries
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_dedup_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("dedup_queries");
    group.sample_size(50);

    let dim = 768;

    // Cenário 1: Poucas queries novas, muitas existentes
    {
        let new = generate_embeddings(10, dim);
        let existing = generate_embeddings(500, dim);

        group.bench_function("10_new_500_existing", |bencher| {
            bencher.iter(|| black_box(dedup_queries(&new, &existing, 0.86)))
        });
    }

    // Cenário 2: Muitas queries novas, poucas existentes
    {
        let new = generate_embeddings(50, dim);
        let existing = generate_embeddings(100, dim);

        group.bench_function("50_new_100_existing", |bencher| {
            bencher.iter(|| black_box(dedup_queries(&new, &existing, 0.86)))
        });
    }

    // Cenário 3: Stress test
    {
        let new = generate_embeddings(100, dim);
        let existing = generate_embeddings(1000, dim);

        group.bench_function("100_new_1000_existing", |bencher| {
            bencher.iter(|| black_box(dedup_queries(&new, &existing, 0.86)))
        });
    }

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Comparação Scalar vs SIMD (Verificação de Acurácia)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_accuracy_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("accuracy");

    let dim = 768;
    let samples = 1000;

    group.bench_function("verify_1000_pairs", |bencher| {
        let pairs: Vec<_> = (0..samples)
            .map(|_| {
                (
                    generate_random_embedding(dim),
                    generate_random_embedding(dim),
                )
            })
            .collect();

        bencher.iter(|| {
            let mut max_error = 0.0f32;
            for (a, b) in pairs.iter() {
                let scalar = cosine_similarity_scalar(a, b);
                let auto = cosine_similarity(a, b);
                let error = (scalar - auto).abs();
                if error > max_error {
                    max_error = error;
                }
            }
            black_box(max_error)
        })
    });

    group.finish();
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// BENCHMARK: Throughput em Batch
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn bench_batch_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_throughput");
    group.sample_size(30);

    let dim = 768;
    let batch_sizes = [100, 500, 1000, 2000];

    for &batch_size in &batch_sizes {
        let query = generate_random_embedding(dim);
        let batch = generate_embeddings(batch_size, dim);

        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("compare_all", batch_size),
            &batch_size,
            |bencher, _| {
                bencher.iter(|| {
                    let results: Vec<f32> =
                        batch.iter().map(|b| cosine_similarity(&query, b)).collect();
                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_cosine_similarity,
    bench_dot_product,
    bench_normalize,
    bench_find_similar,
    bench_dedup_queries,
    bench_accuracy_verification,
    bench_batch_throughput,
);

criterion_main!(benches);
