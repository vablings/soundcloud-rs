use track::Track;
use user::User;

#[derive(Debug, Clone, Deserialize)]
pub enum PlaylistKind {
    #[serde(rename="playlist")]
    Playlist
}

#[derive(Debug, Clone, Deserialize)]
pub enum PlaylistSharing {
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "private")]
    Private
}

#[derive(Debug, Clone, Deserialize)]
pub struct Playlist {
    pub duration: u64,
    pub release_day: Option<i32>,
    pub permalink_url: String,
    pub permalink: String,
    pub purchase_url: Option<String>,
    pub description: Option<String>,
    pub uri: String,
    pub track_count: u64,
    pub user_id: u64,
    pub kind: PlaylistKind,
    pub title: String,
    pub id: u64,
    pub tracks: Vec<Track>,
    pub user: User
}