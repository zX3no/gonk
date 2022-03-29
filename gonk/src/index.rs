#[derive(Debug, Clone)]
pub struct Index<T> {
    pub data: Vec<T>,
    pub index: Option<usize>,
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
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn select(&mut self, i: Option<usize>) {
        self.index = i;
    }
    pub fn is_none(&self) -> bool {
        self.index.is_none()
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<T> Default for Index<T> {
    fn default() -> Self {
        Self {
            data: Vec::default(),
            index: Option::default(),
        }
    }
}
