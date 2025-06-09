use std::ops::{Deref, DerefMut};

pub fn up(len: usize, index: usize, amt: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (index + len - amt % len) % len
}

pub fn down(len: usize, index: usize, amt: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (index + amt) % len
}

#[derive(Debug, PartialEq)]
pub struct Index<T> {
    data: Vec<T>,
    index: Option<usize>,
}

impl<T> Index<T> {
    pub const fn new(data: Vec<T>, index: Option<usize>) -> Self {
        Self { data, index }
    }
    pub fn up(&mut self) {
        if self.data.is_empty() {
            return;
        }

        match self.index {
            Some(0) => self.index = Some(self.data.len() - 1),
            Some(n) => self.index = Some(n - 1),
            None => (),
        }
    }
    pub fn down(&mut self) {
        if self.data.is_empty() {
            return;
        }

        match self.index {
            Some(n) if n + 1 < self.data.len() => self.index = Some(n + 1),
            Some(_) => self.index = Some(0),
            None => (),
        }
    }
    pub fn up_n(&mut self, n: usize) {
        if self.data.is_empty() {
            return;
        }
        let Some(index) = self.index else { return };
        self.index = Some(up(self.data.len(), index, n));
    }
    pub fn down_n(&mut self, n: usize) {
        if self.data.is_empty() {
            return;
        }
        let Some(index) = self.index else { return };
        self.index = Some(down(self.data.len(), index, n));
    }
    pub fn selected(&self) -> Option<&T> {
        let Some(index) = self.index else {
            return None;
        };
        self.data.get(index)
    }
    pub fn selected_mut(&mut self) -> Option<&mut T> {
        let Some(index) = self.index else {
            return None;
        };
        self.data.get_mut(index)
    }
    pub fn index(&self) -> Option<usize> {
        self.index
    }
    pub fn select(&mut self, i: Option<usize>) {
        self.index = i;
    }
    pub fn remove_and_move(&mut self, index: usize) {
        self.data.remove(index);
        let len = self.data.len();
        if let Some(selected) = self.index {
            if index == len && selected == len {
                self.index = Some(len.saturating_sub(1));
            } else if index == 0 && selected == 0 {
                self.index = Some(0);
            } else if len == 0 {
                self.index = None;
            }
        }
    }
}

impl<T> From<Vec<T>> for Index<T> {
    fn from(vec: Vec<T>) -> Self {
        let index = if vec.is_empty() { None } else { Some(0) };
        Self { data: vec, index }
    }
}

impl<'a, T> From<&'a [T]> for Index<&'a T> {
    fn from(slice: &'a [T]) -> Self {
        let data: Vec<&T> = slice.iter().collect();
        let index = if data.is_empty() { None } else { Some(0) };
        Self { data, index }
    }
}

impl<T: Clone> From<&[T]> for Index<T> {
    fn from(slice: &[T]) -> Self {
        let index = if slice.is_empty() { None } else { Some(0) };
        Self {
            data: slice.to_vec(),
            index,
        }
    }
}

impl<T> Default for Index<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            index: None,
        }
    }
}

impl<T> Deref for Index<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> DerefMut for Index<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl crate::Serialize for Index<crate::Song> {
    fn serialize(&self) -> String {
        self.data.serialize()
    }
}
