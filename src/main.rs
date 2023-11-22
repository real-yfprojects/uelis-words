use serde::Deserialize;
use std::env::var;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

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

    std::fs::create_dir_all("wavs")?;

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

        if transcribed
            .iter()
            .filter(|t| t.contains(&episode.created_at))
            .count()
            != 2
        {
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

            split_video(&filename.with_extension("mp4"))?;
        } else {
            println!("`{}` already exists.", episode.id);
        }
    }
    Ok(())
}

fn split_video(filename: &Path) -> anyhow::Result<()> {
    // read `ffmpeg -i filename -af silencedetect=noise=-30dB:d=3.0 -f null - 2> out.txt` and try to split at exactly middle of video
    let output = std::process::Command::new("ffmpeg")
        .arg("-i")
        .arg(filename)
        .arg("-af")
        .arg("silencedetect=noise=-30dB:d=1.0")
        .arg("-f")
        .arg("null")
        .arg("-")
        .output()?;
    if !output.status.success() {
        eprintln!("Failed to get video duration.");
        eprintln!("Status code: {}", output.status);
        eprintln!();
        eprintln!("Response: {:?}", output);
        exit(output.status.code().unwrap_or(1));
    }
    let output = String::from_utf8(output.stderr)?;
    // format is `Duration: 00:00:00.00, start: 0.000000, bitrate: 0 kb/s`
    let duration = output
        .lines()
        .find(|l| l.contains("Duration: ") && l.contains(", start: ") && l.contains(", bitrate: "))
        .expect("Didn't find duration.")
        .split_once("Duration: ")
        .unwrap()
        .1
        .split_once(", start: ")
        .unwrap()
        .0
        .split(':')
        .map(|s| s.parse::<f64>().unwrap())
        .reduce(|a, b| 60.0 * a + b)
        .unwrap();
    let output_lines = output
        .lines()
        .filter(|l| l.contains("silence_end"))
        .collect::<Vec<_>>();
    let middle = output_lines
        .into_iter()
        .map(|l| {
            l.split_once("silence_end: ")
                .unwrap()
                .1
                .split_once(" | silence_duration: ")
                .unwrap()
        })
        .map(|(a, b)| (a.parse::<f64>().unwrap(), b.parse::<f64>().unwrap()))
        .map(|(e, d)| (e - duration / 2.0, d))
        .max_by_key(|(a, _)| a.trunc() as usize)
        .map(|(e, d)| e - d / 2.0)
        .unwrap();

    println!("Splitting at `{middle}`.");

    let output = PathBuf::from("wavs")
        .join(filename.file_name().unwrap())
        .with_extension("wav");

    let p1 = split(filename)
        .arg("-t")
        .arg(format!("{}", middle.round() as usize))
        .arg(output.with_extension("1.wav"))
        .output()?;
    if !p1.status.success() {
        eprintln!("Failed to split video (1).");
        eprintln!("Status code: {}", p1.status);
        eprintln!();
        eprintln!("Response: {:?}", p1);
        exit(p1.status.code().unwrap_or(1));
    }
    println!("Saved part 1.");
    let p2 = split(filename)
        .arg("-ss")
        .arg(format!("{}", middle.round() as usize))
        .arg(output.with_extension("2.wav"))
        .output()?;
    if !p2.status.success() {
        eprintln!("Failed to split video (2).");
        eprintln!("Status code: {}", p2.status);
        eprintln!();
        eprintln!("Response: {:?}", p2);
        exit(p2.status.code().unwrap_or(1));
    }
    println!("Saved part 2.");
    Ok(())
}

fn split(filename: &Path) -> Command {
    let mut command = Command::new("ffmpeg");
    command
        .arg("-i")
        .arg(filename)
        .arg("-acodec")
        .arg("pcm_s16le")
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("16000")
        .arg("-y");
    command
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
