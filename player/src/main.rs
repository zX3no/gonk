#![allow(dead_code)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::Duration,
};

mod windows;
pub use windows::*;

#[derive(Default)]
pub struct Queue<T> {
    q: Arc<Mutex<VecDeque<T>>>,
    cv: Arc<Condvar>,
}

impl<T> Queue<T> {
    /// push input on back of queue
    /// - unrecoverable if lock fails so just unwrap
    pub fn push(&self, t: T) {
        let mut lq = self.q.lock().unwrap();
        lq.push_back(t);
        self.cv.notify_one();
    }
    /// pop element from front of queue
    /// - unrecoverable if lock fails so just unwrap
    /// - same for condition variable
    pub fn pop(&self) -> T {
        let mut lq = self.q.lock().unwrap();
        while lq.len() == 0 {
            lq = self.cv.wait(lq).unwrap();
        }
        lq.pop_front().unwrap()
    }
    pub fn len(&self) -> usize {
        self.q.lock().unwrap().len()
    }
    pub fn is_empty(&self) -> bool {
        self.q.lock().unwrap().is_empty()
    }
}

impl<T> Clone for Queue<T> {
    fn clone(&self) -> Self {
        Self {
            q: self.q.clone(),
            cv: self.cv.clone(),
        }
    }
}

fn queue() {
    let queue = Queue::default();

    let q = queue.clone();

    thread::spawn(move || {
        //Push samples into the queue
        loop {
            thread::sleep(Duration::from_millis(1));
            q.push(0.0);
        }
    });

    loop {
        //Read samples from the queue
        dbg!(queue.pop());
    }
}

fn main() {
    //TODO: Maybe just return the handle and run the stream on creation
    //TODO: Ringbuffer that sends data to the output stream.
    let _handle = create_stream().unwrap();
    thread::park();

    get_default_device();
}
