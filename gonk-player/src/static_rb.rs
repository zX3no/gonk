use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Condvar, Mutex,
};

const MUTEX: Mutex<()> = Mutex::new(());

#[derive(Debug)]
pub struct StaticRb<const N: usize>
where
    [(); N + 1]: Sized,
{
    pub buf: [f32; N + 1],
    pub write: AtomicUsize,
    pub read: AtomicUsize,

    pub slots_requested: AtomicUsize,

    pub block: Condvar,
    pub can_write: AtomicBool,
}

impl<const N: usize> StaticRb<N>
where
    [(); N + 1]: Sized,
{
    pub fn new() -> Self {
        Self {
            buf: [0.0; N + 1],
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
            slots_requested: AtomicUsize::new(0),
            block: Condvar::new(),
            can_write: AtomicBool::new(true),
        }
    }
}

impl<const N: usize> StaticRb<N>
where
    [(); N + 1]: Sized,
{
    pub fn append(&mut self, slice: &[f32]) {
        if slice.is_empty() {
            return;
        }
        if slice.len() > N {
            panic!("Cannot append slice larger than rb size({})", N);
        }

        let write = self.write.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Relaxed);

        let slots_free = if write < read {
            read - write - 1
        } else {
            (N + 1) - write + read
        };

        //Not enough space free.
        if slice.len() > slots_free {
            self.slots_requested.store(slice.len(), Ordering::Relaxed);
            drop(self.block.wait(MUTEX.lock().unwrap()).unwrap());
        }

        //After waiting. Get the write position again.
        let write = self.write.load(Ordering::Relaxed);

        let count = slice.len();
        let buf_len = self.buf.len();
        assert_eq!(buf_len, N + 1);

        if (write + count) < buf_len {
            self.buf[write..write + count].copy_from_slice(&slice[..count]);
        } else {
            let diff = buf_len - write;
            self.buf[write..].copy_from_slice(&slice[..diff]);
            self.buf[..(count - diff)].copy_from_slice(&slice[diff..count]);
        }

        self.write
            .store((write + count) % (N + 1), Ordering::Relaxed);
    }

    pub fn pop(&mut self) -> Option<f32> {
        let write = self.write.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Relaxed);
        if read == write {
            return None;
        }
        let item = self.buf[read];
        self.read.store((read + 1) % (N + 1), Ordering::Relaxed);

        let slots_free = if write < read {
            read - write
        } else {
            (N + 1) - write + read
        };

        let requested = self.slots_requested.load(Ordering::Relaxed);
        if slots_free >= requested {
            self.block.notify_one();
        }

        Some(item)
    }
    pub fn slots_free(&self) -> usize {
        let write = self.write.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Relaxed);

        if write < read {
            read - write
        } else {
            (N + 1) - write + read
        }
    }

    #[inline(always)]
    pub fn read(&self) -> usize {
        self.read.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub fn write(&self) -> usize {
        self.write.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_free() {
        let mut rb = StaticRb::<2>::new();

        rb.append(&[1.0; 1]);
        assert_eq!(rb.write(), 1);
        rb.pop().unwrap();
        assert_eq!(rb.read(), 1);
        rb.append(&[1.0; 1]);
        assert_eq!(rb.write(), 2);
        rb.append(&[1.0; 1]);
        assert_eq!(rb.write(), 0);

        assert!(rb.write() < rb.read());
        assert_eq!(rb.slots_free(), 1);
    }
    #[test]
    fn static_rb() {
        let mut rb = StaticRb::<2>::new();

        rb.append(&[1.0; 2]);
        assert_eq!(rb.buf, [1.0, 1.0, 0.0]);

        assert_eq!(rb.pop().unwrap(), 1.0);
        assert_eq!(rb.pop().unwrap(), 1.0);
        assert_eq!(rb.buf, [1.0, 1.0, 0.0]);

        rb.append(&[2.0; 2]);
        assert_eq!(rb.buf, [2.0, 1.0, 2.0]);

        assert_eq!(rb.pop().unwrap(), 2.0);
        assert_eq!(rb.pop().unwrap(), 2.0);
        assert_eq!(rb.buf, [2.0, 1.0, 2.0]);

        rb.append(&[3.0; 2]);

        assert_eq!(rb.pop().unwrap(), 3.0);
        assert_eq!(rb.pop().unwrap(), 3.0);
        assert_eq!(rb.buf, [2.0, 3.0, 3.0]);

        rb.append(&[4.0; 2]);
        assert_eq!(rb.buf, [4.0, 4.0, 3.0]);
    }
}
