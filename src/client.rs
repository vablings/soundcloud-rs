// Copyright (c) 2016, Mikkel Kroman <mk@uplink.io>
// All rights reserved.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use url::Url;
use reqwest;

use std::result;
use std::borrow::Borrow;
use std::io::{self, Write};

use track::{Track, TrackRequestBuilder, SingleTrackRequestBuilder};
use playlist::Playlist;
use like::Like;
use error::{Error, Result};
use serde_json;

pub type Params<'a, K, V> = &'a [(K, V)];

#[derive(Debug)]
pub struct Client {
    client_id: String,
    auth_token: Option<String>,
    http_client: reqwest::Client,
}

impl Client {
    /// Constructs a new `Client` with the provided `client_id`.
    ///
    /// # Examples
    ///
    /// ```
    /// use soundcloud::Client;
    ///
    /// let client = Client::new(env!("SOUNDCLOUD_CLIENT_ID"));
    /// ```
    pub fn new(client_id: &str) -> Client {
        let client = reqwest::ClientBuilder::new()
            .redirect(reqwest::RedirectPolicy::none())
            .build().unwrap();

        Client {
            client_id: client_id.to_owned(),
            http_client: client,
            auth_token: None,
        }
    }

    /// Returns the client id.
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    pub fn authenticate_with_token(&mut self, token: String) {
        self.auth_token = Some(token);
    }

    /// Creates and sends a HTTP GET request to the API endpoint.
    ///
    /// A `client_id` parameter will automatically be added to the request.
    ///
    /// Returns the HTTP response on success, an error otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Read;
    /// use soundcloud::Client;
    /// let client = Client::new(env!("SOUNDCLOUD_CLIENT_ID"));
    /// let response = client.get("/resolve", Some(&[("url",
    /// "https://soundcloud.com/firepowerrecs/afk-shellshock-kamikaze-promo-mix-lock-load-series-vol-20")]));
    ///
    /// let mut buffer = String::new();
    /// response.unwrap().read_to_string(&mut buffer);
    ///
    /// assert!(!buffer.is_empty());
    /// ```
    pub fn get<I, K, V>(&self, path: &str, params: Option<I>)
        -> result::Result<reqwest::Response, reqwest::Error>
    where I: IntoIterator, I::Item: Borrow<(K, V)>, K: AsRef<str>, V: AsRef<str> {
        let mut url = Url::parse(&format!("https://{}{}", super::API_HOST, path)).unwrap();

        {
            let mut query_pairs = url.query_pairs_mut();
            query_pairs.append_pair("client_id", &self.client_id);

            if let Some(params) = params {
                query_pairs.extend_pairs(params);
            }
        }

        let mut headers = reqwest::header::HeaderMap::new();

        if self.auth_token.is_some() {
            let token = self.auth_token.clone().unwrap();
            headers.insert(reqwest::header::AUTHORIZATION, format!("OAuth {}", token).parse().unwrap());
        }

        let response = self.http_client
            .get(url)
            .headers(headers)
            .send();
        response
    }

    pub fn download<W: Write>(&self, track: &Track, mut writer: W) -> Result<usize> {
        use reqwest::header::LOCATION;

        if !track.downloadable || !track.download_url.is_some() {
            return Err(Error::TrackNotDownloadable);
        }

        let url = self.parse_url(track.download_url.as_ref().unwrap());
        let mut response = self.http_client.get(url).send()?;

        // Follow the redirect just this once.
        if let Some(header) = response.headers().get(LOCATION).cloned() {
            let url = Url::parse(header.to_str()?).unwrap();
            response = self.http_client.get(url).send()?;
        }

        io::copy(&mut response, &mut writer).map(|n| Ok(n as usize))?
    }

    /// Starts streaming the track provided in the tracks `stream_url` to the `writer` if the track
    /// is streamable via the API.
    pub fn stream<W: Write>(&self, track: &Track, mut writer: W) -> Result<usize> {
        use reqwest::header::LOCATION;

        if !track.streamable || !track.stream_url.is_some() {
            return Err(Error::TrackNotStreamable);
        }

        let url = self.parse_url(track.stream_url.as_ref().unwrap());
        let mut response = self.http_client.get(url).send()?;

        // Follow the redirect just this once.
        if let Some(header) = response.headers().get(LOCATION).cloned() {
            let url = Url::parse(header.to_str()?).unwrap();
            response = self.http_client.get(url).send()?;
        }

        io::copy(&mut response, &mut writer).map(|n| Ok(n as usize))?
    }

    /// Resolves any soundcloud resource and returns it as a `Url`.
    pub fn resolve(&self, url: &str) -> Result<Url> {
        use reqwest::header::LOCATION;
        let response = self.get("/resolve", Some(&[("url", url)]))?;

        if let Some(header) = response.headers().get(LOCATION) {
            Ok(Url::parse(header.to_str()?).unwrap())
        } else {
            Err(Error::ApiError("expected location header".to_owned()))
        }
    }

    /// Returns a builder for a single track-by-id request.
    ///
    /// # Examples
    ///
    /// ```
    /// use soundcloud::Client;
    ///
    /// let client = Client::new(env!("SOUNDCLOUD_CLIENT_ID"));
    /// let track = client.track(262681089).get();
    ///
    /// assert_eq!(track.unwrap().id, 262681089);
    /// ```
    pub fn track(&self, id: usize) -> SingleTrackRequestBuilder {
        SingleTrackRequestBuilder::new(self, id)
    }

    /// Returns a builder for searching tracks with multiple criteria.
    ///
    /// # Examples
    ///
    /// ```
    /// use soundcloud::Client;
    ///
    /// let client = Client::new(env!("SOUNDCLOUD_CLIENT_ID"));
    /// let tracks = client.tracks().genres(Some(["HipHop"])).get();
    ///
    /// assert!(tracks.unwrap().expect("no tracks found").len() > 0);
    /// ```
    pub fn tracks(&self) -> TrackRequestBuilder {
        TrackRequestBuilder::new(self)
    }

    pub fn playlists(&self) -> Result<Vec<Playlist>> {
        let params = Some(vec![("limit", "1000")]);
        let res = self.get("/me/playlists", params)?;
        let playlists: Vec<Playlist> = serde_json::from_reader(res)?;
        Ok(playlists)
    }

    pub fn likes(&self) -> Result<Vec<Like>> {
        let params = Some(vec![("limit", "1000")]);
        let res = self.get("/e1/me/likes", params)?;
        let likes: Vec<Like> = serde_json::from_reader(res)?;
        Ok(likes)
    }

    /// Parses a string and returns a url with the client_id query parameter set.
    fn parse_url<S: AsRef<str>>(&self, url: S) -> Url {
        let mut url = Url::parse(url.as_ref()).unwrap();
        url.query_pairs_mut().append_pair("client_id", &self.client_id);
        url
    }
}

#[cfg(test)]
mod tests {
    use url::Url;
    use super::*;

    fn client() -> Client {
        Client::new(env!("SOUNDCLOUD_CLIENT_ID"))
    }

    #[test]
    fn test_fetch_playlists() {
        let mut client = client();
        client.authenticate_with_token(env!("SOUNDCLOUD_AUTH_TOKEN").to_owned());
        assert!(client.playlists().unwrap().len() > 0);
    }

    #[test]
    fn test_fetch_likes() {
        let mut client = client();
        client.authenticate_with_token(env!("SOUNDCLOUD_AUTH_TOKEN").to_owned());
        assert!(client.likes().unwrap().len() > 0);
    }

    #[test]
    fn test_resolve_track() {
        let result = client().resolve("https://soundcloud.com/maxjoehnk/invites-feat-maks-warm-up-mix");

        assert_eq!(result.unwrap(),
            Url::parse(&format!("https://api.soundcloud.com/tracks/330733497?client_id={}",
                                env!("SOUNDCLOUD_CLIENT_ID"))).unwrap());
    }

    #[test]
    fn test_get_tracks() {
        let result = client().tracks().query(Some("d0df0dt snuffx")).get();

        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_get_track() {
        let track = client().tracks().id(18201932).get().unwrap();

        assert_eq!(track.id, 18201932);
    }

    #[test]
    fn test_download_track() {
        use std::fs;
        use std::path::Path;

        let client = client();
        let path = Path::new("hi.mp3");
        let track = client.tracks().id(263801976).get().unwrap();
        let mut file = fs::File::create(path).unwrap();
        let ret = client.download(&track, &mut file);

        assert!(ret.unwrap() > 0);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_stream_track() {
        use std::io::BufWriter;
        let client = client();
        let track = client.tracks().id(262681089).get().unwrap();
        let mut buffer = BufWriter::new(vec![]);
        let len = client.stream(&track, &mut buffer);

        assert!(len.unwrap() > 0);
        assert!(buffer.get_ref().len() > 0);
    }
}
