use std::ops::{Deref, DerefMut};

#[derive(Debug, Default)]
pub struct StaticIndex<T: 'static> {
    slice: &'static [T],
    index: Option<usize>,
}

impl<T> StaticIndex<T> {
    ///Creates a new index.
    ///
    ///If the data is not empty the index will start at `0`.
    pub fn new(slice: &'static [T]) -> Self {
        Self {
            slice,
            index: if slice.is_empty() { Some(0) } else { None },
        }
    }
    pub fn index(&self) -> Option<usize> {
        self.index
    }
    pub fn selected(&self) -> Option<&T> {
        let Some(index) = self.index else {
            return None;
        };
        self.slice.get(index)
    }
    pub fn up(&mut self) {
        if self.slice.is_empty() {
            return;
        }

        match self.index {
            Some(0) => self.index = Some(self.slice.len() - 1),
            Some(n) => self.index = Some(n - 1),
            None => (),
        }
    }
    pub fn down(&mut self) {
        if self.slice.is_empty() {
            return;
        }

        match self.index {
            Some(n) if n + 1 < self.slice.len() => self.index = Some(n + 1),
            Some(_) => self.index = Some(0),
            None => (),
        }
    }
}

impl<T> Deref for StaticIndex<T> {
    type Target = &'static [T];

    fn deref(&self) -> &Self::Target {
        &self.slice
    }
}

impl<T> DerefMut for StaticIndex<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.slice
    }
}

#[derive(Debug)]
pub struct Index<T> {
    data: Vec<T>,
    index: Option<usize>,
}

impl<T> Index<T> {
    pub fn new(data: Vec<T>, index: Option<usize>) -> Self {
        Self { data, index }
    }
    pub fn up(&mut self) {
        if self.data.is_empty() {
            return;
        }

        if let Some(index) = &mut self.index {
            if *index > 0 {
                *index -= 1;
            } else {
                *index = self.data.len() - 1;
            }
        }
    }
    pub fn down(&mut self) {
        if self.data.is_empty() {
            return;
        }

        if let Some(index) = &mut self.index {
            if *index + 1 < self.data.len() {
                *index += 1;
            } else {
                *index = 0;
            }
        }
    }
    pub fn up_with_len(&mut self, len: usize) {
        if let Some(index) = &mut self.index {
            if *index > 0 {
                *index -= 1;
            } else {
                *index = len - 1;
            }
        }
    }
    pub fn down_with_len(&mut self, len: usize) {
        if let Some(index) = &mut self.index {
            if *index + 1 < len {
                *index += 1;
            } else {
                *index = 0;
            }
        }
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
    // pub fn len(&self) -> usize {
    //     self.data.len()
    // }
    pub fn select(&mut self, i: Option<usize>) {
        self.index = i;
    }
    // pub fn is_empty(&self) -> bool {
    //     self.data.is_empty()
    // }
    // pub fn as_slice(&self) -> &[T] {
    //     &self.data
    // }
    pub fn remove_and_move(&mut self, index: usize) {
        self.data.remove(index);
        let len = self.len();
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
