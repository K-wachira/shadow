use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Artist {
    name: String,
}

#[derive(Deserialize, Debug)]
struct Track {
    name: String,
    artists: Vec<Artist>,
}

#[derive(Deserialize, Debug)]
struct PlayHistoryItem {
    track: Track,
    played_at: String,
}

#[derive(Deserialize, Debug)]
struct RecentlyPlayedResponse {
    items: Vec<PlayHistoryItem>,
}

#[tokio::main]
async fn spotify() {
    let token = "YOUR_ACCESS_TOKEN"; // from OAuth flow

    let client = Client::new();
    let res: RecentlyPlayedResponse = client
        .get("https://api.spotify.com/v1/me/player/recently-played")
        .query(&[("limit", "20")])
        .bearer_auth(token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    for item in res.items {
        println!(
            "{} — {} @ {}",
            item.track.name, item.track.artists[0].name, item.played_at
        );
    }
}
