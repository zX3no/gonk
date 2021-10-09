use crate::database::album::Album;
pub struct Artist {
    name: String,
    albums: Vec<Album>,
}
