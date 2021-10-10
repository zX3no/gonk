use crate::database::album::Album;

#[derive(Debug, Clone)]
pub struct Artist {
    pub name: String,
    pub albums: Vec<Album>,
}

impl Artist {
    pub fn album(&self, name: &str) -> Option<&Album> {
        let mut out = None;
        for album in &self.albums {
            if album.title == name {
                out = Some(album);
            }
        }
        return out;
    }
}
