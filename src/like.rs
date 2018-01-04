use track::Track;
use playlist::Playlist;

#[derive(Debug, Deserialize, Clone)]
pub struct Like {
    pub track: Option<Track>,
    pub playlist: Option<Playlist>
}