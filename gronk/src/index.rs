//TODO: would be nice to have a type that can handle all selectable data
//data: Vec<T>
//index: Option<usize>
pub struct Index {
    pub index: Option<usize>,
}
impl Index {
    pub fn new() -> Self {
        Self { index: None }
    }
    pub fn up(&mut self, len: usize) {
        if let Some(index) = &mut self.index {
            if *index > 0 {
                *index -= 1;
            } else {
                *index = len - 1;
            }
        }
    }
    pub fn down(&mut self, len: usize) {
        if let Some(index) = &mut self.index {
            if *index + 1 < len {
                *index += 1;
            } else {
                *index = 0;
            }
        }
    }
    pub fn select(&mut self, i: Option<usize>) {
        self.index = i;
    }
    pub fn is_none(&self) -> bool {
        self.index.is_none()
    }
}
