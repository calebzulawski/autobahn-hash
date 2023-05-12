use core::hash::{Hash, Hasher};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn slice(c: &mut Criterion) {
    let mut group = c.benchmark_group("slice");
    for size in 1..16 {
        let size = 2 << size;
        group.throughput(Throughput::Bytes(size));

        // random-ish input
        let input = (0..size)
            .map(|x| ((x & 0xff) ^ 0xe4) as u8)
            .collect::<Vec<u8>>();

        // random key
        let key = [
            0x16384296d6154fbb,
            0xae37b154c5f60eeb,
            0xd73f631cd1808f01,
            0xb37d9add6cc091a9,
        ];

        group.bench_with_input(
            BenchmarkId::new("default", size),
            input.as_ref(),
            |b, input| {
                b.iter(|| {
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    hasher.write(black_box(input));
                    black_box(hasher.finish());
                });
            },
        );

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

fn hasher(c: &mut Criterion) {
    #[derive(Hash)]
    struct Data(u8, u16, u32, u64);

    impl Data {
        fn new(i: usize) -> Self {
            let i = i as u64;
            Self(
                (i ^ 0x66919bf6b0982f0c & 0xff) as u8,
                (i ^ 0x421350846684be73 & 0xffff) as u16,
                (i ^ 0x3fbf0c820d73bcb0 & 0xffffffff) as u32,
                i ^ 0x8e6c29addc970986,
            )
        }
    }

    let mut group = c.benchmark_group("struct");
    for size in 1..8 {
        let size = 2 << size;
        group.throughput(Throughput::Bytes(
            (size * core::mem::size_of::<Data>()) as u64,
        ));

        // random-ish input
        let input = (0..size).map(Data::new).collect::<Vec<Data>>();

        group.bench_with_input(
            BenchmarkId::new("default", size),
            input.as_ref(),
            |b, input: &[Data]| {
                b.iter(|| {
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    input.hash(&mut hasher);
                    black_box(hasher.finish());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("highway", size),
            input.as_ref(),
            |b, input: &[Data]| {
                b.iter(|| {
                    let mut hasher = highway::HighwayHasher::new(highway::Key([0; 4]));
                    input.hash(&mut hasher);
                    black_box(hasher.finish());
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("autobahn-hash", size),
            input.as_ref(),
            |b, input: &[Data]| {
                b.iter(|| {
                    let mut hasher = autobahn_hash::AutobahnHasher::new();
                    input.hash(&mut hasher);
                    black_box(hasher.finish());
                });
            },
        );
    }
    group.finish()
}

criterion_group!(benches, slice, hasher);
criterion_main!(benches);
