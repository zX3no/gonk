#[derive(Debug)]
pub struct List {
    pub items: Vec<String>,
    //todo maybe remove
    pub selection: usize,
    pub len: usize,
}
impl List {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selection: 0,
            len: 0,
        }
    }
    pub fn from_vec(items: Vec<String>) -> Self {
        let len = items.len().checked_sub(1).unwrap_or(0);
        Self {
            items,
            selection: 0,
            len,
        }
    }
    pub fn filter(&mut self, query: &String) {
        self.items = self
            .items
            .iter()
            .filter_map(|item| {
                if item.to_lowercase().contains(query) {
                    return Some(item.clone());
                }
                None
            })
            .collect();

        //reset the data
        self.len = self.items.len().checked_sub(1).unwrap_or(0);
        self.selection = 0;
    }
    pub fn down(&mut self) {
        // dbg!(self.len, self.selection);
        if self.selection != self.len {
            self.selection += 1;
        } else {
            self.selection = 0;
        }
    }
    pub fn up(&mut self) {
        if self.selection != 0 {
            self.selection -= 1;
        } else {
            self.selection = self.len;
        }
    }
    pub fn selected(&self) -> String {
        self.items.get(self.selection).unwrap().clone()
    }
    pub fn clear_selection(&mut self) {
        self.selection = 0;
    }
}
