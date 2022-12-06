#[derive(Debug)]
pub struct Index<T> {
    pub data: Vec<T>,
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
        if let Some(index) = self.index {
            if let Some(item) = self.data.get(index) {
                return Some(item);
            }
        }
        None
    }
    pub fn selected_mut(&mut self) -> Option<&mut T> {
        if let Some(index) = self.index {
            if let Some(item) = self.data.get_mut(index) {
                return Some(item);
            }
        }
        None
    }
    pub fn index(&self) -> Option<usize> {
        self.index
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn select(&mut self, i: Option<usize>) {
        self.index = i;
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }
    pub fn remove(&mut self, index: usize) {
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
        Self::new(vec, index)
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
