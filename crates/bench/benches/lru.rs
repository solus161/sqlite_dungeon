use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

use bench::{lrucache_key::LRUCacheKey, lrucache_rc::LRUCacheRc, lrucache_vec::LRUCacheVec};

/// Simple wrapper, as LRUCacheRc has distinct output for `get` fn
trait CommonCache {
    fn new(capacity: usize) -> Self;

    fn get(&mut self, k: &u64);

    fn set(&mut self, k: u64, v: Vec<u8>);

}

impl CommonCache for LRUCacheKey {
    fn new(capacity: usize) -> Self {
        Self::new(capacity)
    }

    fn get(&mut self, k: &u64) {
        let _ = Self::get(self, k);
    }

    fn set(&mut self, k: u64, v: Vec<u8>) {
        Self::set(self, k, v);
    }
}

impl CommonCache for LRUCacheRc {
    fn new(capacity: usize) -> Self {
        Self::new(capacity)
    }

    fn get(&mut self, k: &u64) {
        Self::get(self, k);
    }

    fn set(&mut self, k: u64, v: Vec<u8>) {
        Self::set(self, k, v);
    }
}

impl CommonCache for LRUCacheVec {
    fn new(capacity: usize) -> Self {
        Self::new(capacity)
    }

    fn get(&mut self, k: &u64) {
        let _ = Self::get(self, k);
    }

    fn set(&mut self, k: u64, v: Vec<u8>) {
        Self::set(self, k, v);
    }
}

const CAP: u64 = 1024;

fn fill_cache<C: CommonCache>() -> C {
    let mut cache = C::new(CAP as usize);
    for k in 0..CAP {
        cache.set(k, vec![0u8;64]);
    };
    cache
}

fn bench_impl<C: CommonCache>(c: &mut Criterion, name: &str) {
    let mut g = c.benchmark_group(name);

    g.bench_function("get_hit", |b| {
        let mut cache = fill_cache::<C>();
        let mut i = 0u64;
        b.iter(|| {
            i = (i + 1) % CAP;
            black_box(cache.get(&i))
        })
    });

    g.bench_function("get_miss", |b| {
        let mut cache = fill_cache::<C>();
        let mut i = CAP;
        b.iter(|| {
            i += 1;
            black_box(cache.get(&i))
        });
    });

    g.bench_function("set_evict", |b| {
        let mut cache = fill_cache::<C>();
        let mut k = CAP;
        let page = vec![0u8; 64];
        b.iter(|| {
            k += 1;
            black_box(cache.set(k, page.clone()))
        });
    });

    g.finish();
}

fn benches(c: &mut Criterion) {
    bench_impl::<LRUCacheKey>(c, "LRUCacheKey");
    bench_impl::<LRUCacheRc>(c, "LRUCacheRc");
    bench_impl::<LRUCacheVec>(c, "LRUCacheVec");
}

criterion_group!(lru, benches);
criterion_main!(lru);
