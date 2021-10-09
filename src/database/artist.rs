use crate::database::album::Album;

#[derive(Debug, Clone)]
pub struct Artist {
    pub name: String,
    pub albums: Vec<Album>,
}
