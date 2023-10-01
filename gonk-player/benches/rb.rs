use criterion::*;
pub use gonk_player::*;

const ADD: [u32; 1024] = [10; 1024];

fn for_loop(arr: &mut [u32]) {
    for (i, item) in ADD.iter().enumerate() {
        arr[i] = *item;
    }
}

fn ptr_write(arr: &mut [u32]) {
    for (i, item) in ADD.iter().enumerate() {
        unsafe { std::ptr::write(arr.as_mut_ptr().add(i), *item) };
    }
}

fn batch(arr: &mut [u32]) {
    arr.copy_from_slice(&ADD);
}

fn read(arr: &mut [u32]) {
    for i in 0..arr.len() {
        assert_eq!(arr[i], 10);
    }
}

fn read_ptr(arr: &mut [u32]) {
    for i in 0..arr.len() {
        assert_eq!(unsafe { std::ptr::read(arr.as_mut_ptr().add(i)) }, 10);
    }
}

fn loop_vs_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("loop_vs_batch");
    group.sample_size(100);

    group.bench_function("for", |b| {
        let mut arr = [0; 1024];
        b.iter(|| black_box(for_loop(&mut arr)));
    });

    group.bench_function("batch", |b| {
        let mut arr = [0; 1024];
        b.iter(|| black_box(batch(&mut arr)));
    });

    group.bench_function("ptr_write", |b| {
        let mut arr = [0; 1024];
        b.iter(|| black_box(ptr_write(&mut arr)));
    });

    group.bench_function("read", |b| {
        let mut arr = [10; 1024];
        b.iter(|| black_box(read(&mut arr)));
    });

    group.bench_function("read_ptr", |b| {
        let mut arr = [10; 1024];
        b.iter(|| black_box(read_ptr(&mut arr)));
    });

    group.finish();
}

criterion_group!(benches, loop_vs_batch);
criterion_main!(benches);
