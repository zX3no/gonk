use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Condvar, Mutex,
};

#[derive(Debug)]
pub struct Rb<T: Default + Clone, const N: usize>
where
    [(); N + 1]: Sized,
{
    /// Note that N-1 is the actual capacity.
    pub buf: Mutex<[T; N + 1]>,
    // pub buf: [T; N + 1],
    pub write: AtomicUsize,
    pub read: AtomicUsize,
    pub block: Condvar,
    pub can_write: AtomicBool,
}

impl<const N: usize> Rb<f32, N>
where
    [(); N + 1]: Sized,
{
    pub const fn new() -> Self {
        Self {
            buf: Mutex::new([0.0; (N + 1)]),
            // buf: [0.0; (N + 1)],
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
            block: Condvar::new(),
            can_write: AtomicBool::new(true),
        }
    }
}

const MUTEX: Mutex<()> = Mutex::new(());

impl<T: Default + Clone, const N: usize> Rb<T, N>
where
    [(); N + 1]: Sized,
{
    pub fn push_blocking(&mut self, value: T) {
        let write = self.write.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Relaxed);
        let guard = self.buf.lock().unwrap();
        let mut buf = if (write + 1) % (N + 1) == read {
            //Wait for read to free up space.
            self.block.wait(guard).unwrap()
        } else {
            guard
        };
        unsafe { std::ptr::write(buf.as_mut_ptr().add(write), value) };
        self.write.store((write + 1) % (N + 1), Ordering::Relaxed);
    }

    pub fn pop(&mut self) -> Option<T> {
        let write = self.write.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Relaxed);
        if read == write {
            return None;
        }
        let mut buf = self.buf.lock().unwrap();
        //Don't allow write before reading.
        let item = unsafe { std::ptr::read(buf.as_mut_ptr().add(read)) };
        self.read.store((read + 1) % (N + 1), Ordering::Relaxed);
        self.block.notify_one();
        Some(item)
    }

    // pub fn push_blocking(&mut self, value: T) {
    //     let write = self.write.load(Ordering::Relaxed);
    //     let read = self.read.load(Ordering::Relaxed);
    //     if (write + 1) % (N + 1) == read {
    //         //Wait for read to free up space.
    //         drop(self.block.wait(MUTEX.lock().unwrap()).unwrap());
    //     };
    //     unsafe { std::ptr::write(self.buf.as_mut_ptr().add(write), value) };
    //     self.write.store((write + 1) % (N + 1), Ordering::Relaxed);
    // }

    // pub fn pop(&mut self) -> Option<T> {
    //     let write = self.write.load(Ordering::Relaxed);
    //     let read = self.read.load(Ordering::Relaxed);
    //     if read == write {
    //         return None;
    //     }
    //     //Don't allow write before reading.
    //     let item = unsafe { std::ptr::read(self.buf.as_mut_ptr().add(read)) };
    //     self.read.store((read + 1) % (N + 1), Ordering::Relaxed);
    //     self.block.notify_one();
    //     Some(item)
    // }
}
