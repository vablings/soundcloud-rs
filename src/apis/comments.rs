use futures::stream::BoxStream;

use crate::client::Client;
use crate::error::Result;
use crate::models::Comment;
use crate::streaming_api::StreamingApi;

/// Provides access to operations available for comments
pub struct Comments {
    client: Client,
    track_id: usize,
}

impl StreamingApi for Comments {
    type Model = Comment;

    fn path(&self) -> String {
        format!("/tracks/{}/comments", self.track_id)
    }

    fn get_stream(&self, url: &str, pages: Option<u64>) -> BoxStream<'_, Result<Self::Model>> {
        self.client.get_stream(url, pages)
    }
}

impl Comments {
    /// create a new instance of a souncloud track's comments
    pub fn track(client: Client, track_id: usize) -> Self {
        Comments { client, track_id }
    }
}
