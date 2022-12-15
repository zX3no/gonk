//! TODO: Cleanup
//!
//!
use crate::Lazy;
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::RwLock,
    time::{Duration, Instant},
};

static mut EVENTS: Lazy<RwLock<HashMap<Location, Vec<Event>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Clone, Debug, Default)]
struct Event {
    pub start: Option<Instant>,
    pub end: Option<Instant>,
}

impl Event {
    pub unsafe fn elapsed(&self) -> Duration {
        self.end
            .unwrap_unchecked()
            .duration_since(self.start.unwrap_unchecked())
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Debug, Default)]
struct Location {
    pub name: &'static str,
    pub file: &'static str,
    pub line: u32,
}

#[derive(Debug)]
struct Score {
    pub name: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub mean: Duration,
    pub min: Duration,
    pub max: Duration,
    pub count: usize,
}

struct Dropper {
    pub event: Event,
    pub location: Location,
}

impl Drop for Dropper {
    #[inline(always)]
    fn drop(&mut self) {
        let mut map = unsafe { EVENTS.write().unwrap() };
        self.event.end = Some(Instant::now());
        match map.entry(self.location.clone()) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push(self.event.clone());
            }
            Entry::Vacant(entry) => {
                entry.insert(vec![self.event.clone()]);
            }
        }
    }
}

#[macro_export]
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        &name[..name.len() - 3]
    }};
}

//HACK: cfg doesn't work in macros.
#[inline(always)]
pub fn profile(_name: &'static str) {
    #[cfg(feature = "profile")]
    let _drop = Dropper {
        event: Event {
            start: Some(std::time::Instant::now()),
            end: None,
        },
        location: Location {
            name: _name,
            file: file!(),
            line: line!(),
        },
    };
}

#[macro_export]
macro_rules! profile {
    () => {
        $crate::profiler::profile($crate::function!());
    };
    ($name:expr) => {
        $crate::profiler::profile($name);
    };
}

///Print the profiler events.
pub fn print() {
    let events = unsafe { EVENTS.read().unwrap() };

    if events.is_empty() {
        return;
    }

    let mut scores = Vec::new();

    for (k, v) in events.iter() {
        let mut mean = Duration::default();
        let mut min = Duration::default();
        let mut max = Duration::default();

        for event in v {
            let elapsed = unsafe { event.elapsed() };

            if elapsed < min {
                min = elapsed;
            }

            if elapsed > max {
                max = elapsed;
            }

            mean += elapsed;
        }

        scores.push(Score {
            name: k.name,
            file: k.file,
            line: k.line,
            mean: mean.checked_div(v.len() as u32).unwrap_or_default(),
            min,
            max,
            count: v.len(),
        });
    }

    for score in scores {
        println!(
            "{} ({} runs) {}:{}",
            score.name, score.count, score.file, score.line,
        );
        println!("   - mean: {:?}", score.mean);
        println!("   - min: {:?}", score.min);
        println!("   - max: {:?}\n", score.max);
    }
}
