use std::ops::{Deref, DerefMut};

//TODO: There is probably a more generic way to do both
#[derive(Debug)]
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
            index: if slice.is_empty() { None } else { Some(0) },
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

impl<T> Default for StaticIndex<T> {
    fn default() -> Self {
        Self {
            slice: &[],
            index: None,
        }
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
    pub fn select(&mut self, i: Option<usize>) {
        self.index = i;
    }
    //TODO: This is only used once maybe remove
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
        // let _ = self.remove(index);
        // let len = self.len();
        // match self.index {
        //     Some(selected) if selected == 0 && index == 0 => self.index = Some(0),
        //     Some(_) if len == 0 => self.index = None,
        //     _ => self.index = Some(len.saturating_sub(1)),
        // }
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

impl crate::Serialize for Index<crate::Song> {
    fn serialize(&self) -> String {
        self.data.serialize()
    }
}
