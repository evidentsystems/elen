//! ELEN against plain decimal text and a binary order-preserving encoding (ordcode), for `u64`
//! and `i64` values across magnitudes.

use std::{fmt::Write, hint::black_box};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ordcode::{Order, de_from_bytes_ordered, ser_to_buf_ordered};

const U64S: &[(&str, u64)] = &[("small", 7), ("medium", 1_234_567), ("max", u64::MAX)];
const I64S: &[(&str, i64)] = &[
    ("neg-small", -7),
    ("neg-medium", -1_234_567),
    ("min", i64::MIN),
];

fn encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode");
    let mut s = String::with_capacity(64);
    let mut buf = [0u8; 16];
    for &(name, v) in U64S {
        group.bench_with_input(BenchmarkId::new("elen/u64", name), &v, |b, &v| {
            b.iter(|| {
                s.clear();
                elen::Encode::encode_into(&black_box(v), &mut s).unwrap();
            })
        });
        group.bench_with_input(BenchmarkId::new("decimal/u64", name), &v, |b, &v| {
            b.iter(|| {
                s.clear();
                write!(s, "{}", black_box(v)).unwrap();
            })
        });
        group.bench_with_input(BenchmarkId::new("ordcode/u64", name), &v, |b, &v| {
            b.iter(|| ser_to_buf_ordered(&mut buf, &black_box(v), Order::Ascending).unwrap())
        });
    }
    for &(name, v) in I64S {
        group.bench_with_input(BenchmarkId::new("elen/i64", name), &v, |b, &v| {
            b.iter(|| {
                s.clear();
                elen::Encode::encode_into(&black_box(v), &mut s).unwrap();
            })
        });
        group.bench_with_input(BenchmarkId::new("ordcode/i64", name), &v, |b, &v| {
            b.iter(|| ser_to_buf_ordered(&mut buf, &black_box(v), Order::Ascending).unwrap())
        });
    }
    group.finish();
}

fn decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");
    for &(name, v) in U64S {
        let e = elen::encode(&v);
        let d = v.to_string();
        let mut o = [0u8; 16];
        let n = ser_to_buf_ordered(&mut o, &v, Order::Ascending).unwrap();
        group.bench_with_input(BenchmarkId::new("elen/u64", name), &e, |b, e| {
            b.iter(|| elen::decode::<u64>(black_box(e)).unwrap())
        });
        group.bench_with_input(BenchmarkId::new("decimal/u64", name), &d, |b, d| {
            b.iter(|| black_box(d).parse::<u64>().unwrap())
        });
        group.bench_with_input(BenchmarkId::new("ordcode/u64", name), &o[..n], |b, o| {
            b.iter(|| {
                let mut bytes = [0u8; 16];
                bytes[..o.len()].copy_from_slice(black_box(o));
                de_from_bytes_ordered::<_, u64>(&mut bytes[..o.len()], Order::Ascending).unwrap()
            })
        });
    }
    group.finish();
}

criterion_group!(benches, encode, decode);
criterion_main!(benches);
