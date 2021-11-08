pub struct Search {
    pub query: String,
}
impl Search {
    pub fn new() -> Self {
        Self {
            query: String::new(),
        }
    }
}
