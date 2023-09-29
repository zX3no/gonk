use log::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Condvar, Mutex,
};

#[derive(Debug)]
pub struct Rb<T: Default + Clone, const N: usize>
where
    [(); N + 1]: Sized,
{
    /// Note that N-1 is the actual capacity.
    pub buf: [T; N + 1],
    pub write: AtomicUsize,
    pub read: AtomicUsize,
    pub block: Condvar,
    pub locked: Mutex<bool>,
}

impl<const N: usize> Rb<f32, N>
where
    [(); N + 1]: Sized,
{
    pub const fn new() -> Self {
        Self {
            buf: [0.0; (N + 1)],
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
            block: Condvar::new(),
            locked: Mutex::new(false),
        }
    }
}

// impl<const N: usize> Rb<u8, N>
// where
//     [u8; N + 1]: Sized,
// {
//     pub const fn new() -> Self {
//         Self {
//             buf: [0; (N + 1)],
//             write: AtomicUsize::new(0),
//             read: AtomicUsize::new(0),
//             block: Condvar::new(),
//             locked: Mutex::new(false),
//         }
//     }
// }

impl<T: Default + Clone, const N: usize> Rb<T, N>
where
    [(); N + 1]: Sized,
{
    pub fn push_back(&mut self, value: T) {
        let write = self.write.load(Ordering::SeqCst);
        let read = self.read.load(Ordering::SeqCst);
        info!("Pushing read: {}, write: {}", read, write);

        if (write + 1) % (N + 1) == read {
            //Wait for read to free up space.
            let mut locked = self.locked.lock().unwrap();
            *locked = true;
            info!("Buffer full. Blocking thread. locked: {}", *locked);
            drop(self.block.wait(locked).unwrap());

            //Try again.
            return self.push_back(value);
        }

        self.write(write, value);
        self.write.store((write + 1) % (N + 1), Ordering::SeqCst);
    }

    pub fn pop_front(&mut self) -> Option<T> {
        let write = self.write.load(Ordering::SeqCst);
        let read = self.read.load(Ordering::SeqCst);
        info!("Pop read: {}, write: {}", read, write);

        if read == write {
            None
        } else {
            //Don't allow write before reading.
            let item = self.read(read);
            self.read.store((read + 1) % (N + 1), Ordering::SeqCst);

            //Notify the pushing thread to wake up.
            let mut locked = self.locked.lock().unwrap();
            if *locked {
                *locked = false;
                info!("Notifying all threads to unlock. locked: {}", *locked);
                self.block.notify_all();
            }

            Some(item)
        }
    }

    /// Writes an element into the buffer, moving it.
    #[inline]
    fn write(&mut self, off: usize, value: T) {
        unsafe {
            std::ptr::write(self.buf.as_mut_ptr().add(off), value);
        }
    }

    /// Read an element without moving it.
    #[inline]
    fn read(&mut self, off: usize) -> T {
        unsafe { std::ptr::read(self.buf.as_mut_ptr().add(off)) }
    }
}
