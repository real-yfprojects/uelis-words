use serde::Deserialize;
use std::env::var;
use std::fs::File;
use std::path::PathBuf;
use std::process::exit;

use reqwest::blocking::Client;

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
struct Metadata {
    authorized: bool,
    episodes: Vec<Episode>,
    id: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
struct Episode {
    #[serde(rename = "createdAt")]
    created_at: String,
    id: String,
}

fn main() -> anyhow::Result<()> {
    let username = var("USERNAME").expect("USERNAME not set");
    let password = var("PASSWORD").expect("PASSWORD not set");

    let client = Client::builder().cookie_store(true).build()?;

    println!("Logging in ...");

    let res = client
        .post("https://video.ethz.ch/lectures/d-infk/2023/autumn/252-0025-01L.series-login.json")
        .form(&[
            ("_charset_", "utf-8"),
            ("username", &username),
            ("password", &password),
        ])
        .send()?;
    if !res.status().is_success() {
        eprintln!("Failed to login.");
        eprintln!("Status code: {}", res.status());
        eprintln!();
        eprintln!("Response: {:?}", res);
        exit(-1);
    }

    println!("Logged in.");

    println!("Getting metadata ...");

    let metadata: Metadata = client
        .get("https://video.ethz.ch/lectures/d-infk/2023/autumn/252-0025-01L.series-metadata.json")
        .send()?
        .json()?;

    assert!(metadata.authorized, "Not authorized to access this series.");

    println!("Found {} episode(s).", metadata.episodes.len());

    let transcribed = std::fs::read_to_string("transcribed.txt")?
        .lines()
        .map(|s| s.to_string())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>();

    for episode in metadata.episodes {
        let links: EpisodeMetadata = client
            .get(&format!(
				"https://video.ethz.ch/lectures/d-infk/2023/autumn/252-0025-01L/{}.series-metadata.json",
				episode.id
			))
            .send()?
            .json()?;
        let video_links = links
            .selected_episode
            .media
            .presentations
            .into_iter()
            .filter(|p| p.mime_type == "video/mp4")
            .collect::<Vec<_>>();
        assert!(
            !video_links.is_empty(),
            "No video links for `{}` found.",
            episode.id
        );
        let worst = video_links
            .into_iter()
            .min_by_key(|p| p.width * p.height)
            .unwrap();

        let videos = PathBuf::from("videos");
        let mut filename = videos.join(&episode.created_at);
        filename.set_extension("mp4");

        if transcribed.iter().all(|t| !t.contains(&episode.created_at)) {
            filename.set_extension("tmp");

            println!(
                "Downloading `{}` to `{}`...",
                episode.id, episode.created_at
            );

            std::fs::create_dir_all(&videos)?;

            client
                .get(worst.url)
                .send()?
                .copy_to(&mut File::create(&filename)?)?;

            std::fs::rename(&filename, filename.with_extension("mp4"))?;
            println!("Downloaded `{}`.", episode.id);
        } else {
            println!("`{}` already exists.", episode.id);
        }
    }
    Ok(())
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
struct EpisodeMetadata {
    #[serde(rename = "selectedEpisode")]
    selected_episode: SelectedEpisode,
}
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
struct SelectedEpisode {
    media: Media,
}
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
struct Media {
    presentations: Vec<Presentation>,
}
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
struct Presentation {
    width: usize,
    height: usize,
    url: String,
    #[serde(rename = "type")]
    mime_type: String,
}
