use cachedhash::cachedhash::CachedHash;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::fmt::Formatter;
use std::hash::Hash;
use std::{collections::HashMap, fmt::Display};

#[inline]
fn move_things_around<T: Hash + Eq>(
    map1: &mut HashMap<T, ()>,
    map2: &mut HashMap<T, ()>,
    steps: usize,
) {
    for _ in 0..steps {
        for (key, value) in map1.drain() {
            map2.insert(key, value);
        }
        for (key, value) in map2.drain() {
            map1.insert(key, value);
        }
    }
}

struct Param(usize, usize, usize);

impl Display for Param {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.0, self.1, self.2)
    }
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("A");
    for &map_size in [100, 1000, 10000].iter() {
        for &word_length in [5, 10, 20, 100].iter() {
            for &steps in [1, 5, 10, 20].iter() {
                let mut data = vec![];
                for j in 0..map_size {
                    data.push(j.to_string().repeat(word_length as usize));
                }
                bench_hashmap("Regular", &mut group, map_size, word_length, steps, &data);
                let data = data
                    .into_iter()
                    .map(|s| CachedHash::new(s))
                    .collect::<Vec<_>>();
                bench_hashmap("Cached", &mut group, map_size, word_length, steps, &data);
            }
        }
    }
    group.finish();
}

fn bench_hashmap<T: Eq + Hash + Clone>(
    name: &str,
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
    map_size: usize,
    word_length: usize,
    steps: usize,
    data: &Vec<T>,
) {
    group.bench_with_input(
        BenchmarkId::new(name, Param(map_size, word_length, steps)),
        data,
        |b, data| {
            b.iter(|| {
                let mut map = HashMap::new();
                for key in data {
                    map.insert(key.clone(), ());
                }
                move_things_around(&mut map, &mut HashMap::new(), steps);
            })
        },
    );
}

criterion_group!(benches, bench);
criterion_main!(benches);
