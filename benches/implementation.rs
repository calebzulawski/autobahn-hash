use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn implementation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Implementation");
    for size in 1..16 {
        let size = 2 << size;
        group.throughput(Throughput::Bytes(size));

        // random-ish input
        let input = (0..size).map(|x| x as u8 ^ 0xe4).collect::<Vec<u8>>();

        // random key
        let key = [
            0x16384296d6154fbb,
            0xae37b154c5f60eeb,
            0xd73f631cd1808f01,
            0xb37d9add6cc091a9,
        ];

        group.bench_with_input(
            BenchmarkId::new("highway", size),
            input.as_ref(),
            |b, input| {
                b.iter(|| {
                    use highway::{HighwayHash, HighwayHasher};
                    let mut hasher = HighwayHasher::new(highway::Key(black_box(key)));
                    hasher.append(black_box(input));
                    black_box(hasher.finalize64());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("autobahn-hash", size),
            input.as_ref(),
            |b, input| {
                b.iter(|| {
                    black_box(autobahn_hash::hash_u64(black_box(input), black_box(key)));
                });
            },
        );
    }
    group.finish()
}

criterion_group!(benches, implementation);
criterion_main!(benches);
