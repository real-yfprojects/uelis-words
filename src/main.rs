use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use std::thread::available_parallelism;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperError};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::env::var;
use std::fs::File;
use std::process::{exit, Command};

use reqwest::blocking::Client;

use std::env::temp_dir;
use std::fs::remove_file;
use std::io::{self, Write};
use std::path::PathBuf;

use hound::WavReader;

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
    let last_seen =
        NaiveDateTime::parse_from_str(&var("LAST_SEEN").expect("LAST_SEEN not set"), "%FT%R")
            .expect(
                "Failed to parse `LAST_SEEN`. Make sure to use the pseudo ISO: `YYYY-mm-ddTHH:MM`",
            );

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

    let episodes = metadata
        .episodes
        .iter()
        .filter(|e| NaiveDateTime::parse_from_str(&e.created_at, "%FT%R").unwrap() > last_seen)
        .collect::<Vec<_>>();

    println!("Found {} new episode(s).", episodes.len());

    for episode in episodes {
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
        let filename = temp_dir().join(&episode.id);

        println!("Downloading `{}` ...", episode.id);

        client
            .get(worst.url)
            .send()?
            .copy_to(&mut File::create(&filename)?)?;

        println!("Downloaded `{}`.", episode.id);

        println!("Transcribing ...");
        let audio = generate_audio(&filename)?;
        let segments = Whisper::new("whisper.model")?.transcribe(audio)?;

        println!("Done!");

        println!("Saving ...");
        serde_json::to_writer_pretty(File::create("output.json")?, &segments)?;
        println!("Saved!");
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

fn read_wav(p: PathBuf) -> io::Result<Vec<f32>> {
    println!("{p:?}");

    let wav = WavReader::open(&p)
        .unwrap()
        .into_samples()
        .map(|r| r.unwrap())
        .collect();
    remove_file(p)?;

    Ok(wav)
}

pub fn generate_audio<P: AsRef<Path>>(p: P) -> std::io::Result<Vec<f32>> {
    let mut out = temp_dir().join(p.as_ref().file_name().unwrap());
    out.set_extension("wav");

    let output = Command::new("ffmpeg")
        .args([
            "-i",
            &p.as_ref().to_str().unwrap(),
            "-ac",
            "1",
            "-ar",
            "16000",
            "-acodec",
            "pcm_f32le",
            "-f",
            "wav",
            out.to_str().unwrap(),
            "-y",
        ])
        .output()?;
    if !output.status.success() {
        println!("======ffmpeg error=====");
        println!("stdout:");
        std::io::stdout().write_all(&output.stdout)?;
        println!();
        eprintln!("stderr:");
        std::io::stderr().write_all(&output.stderr)?;
        std::process::exit(1);
    }
    read_wav(out)
}

pub type WhisperResult<T> = Result<T, WhisperError>;

pub struct Whisper(Arc<WhisperContext>);

impl Whisper {
    fn get_ctx<P: AsRef<Path>>(path_to_model: P) -> WhisperResult<WhisperContext> {
        let path = path_to_model.as_ref().display().to_string();
        WhisperContext::new(&path)
    }
}

impl Whisper {
    pub fn new<P: AsRef<Path>>(path_to_model: P) -> WhisperResult<Self> {
        Self::get_ctx(path_to_model).map(Arc::new).map(Self)
    }

    pub fn transcribe(&self, audio_data: Vec<f32>) -> WhisperResult<Vec<Segment>> {
        let ctx = self.0.clone();

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("de"));
        params.set_print_progress(true);
        params.set_print_realtime(true);
        params.set_print_timestamps(true);
        params.set_print_special(false);
        params
            .set_n_threads(8.min(available_parallelism().map(NonZeroUsize::get).unwrap_or(1)) as _);

        let mut state = ctx.create_state()?;

        state.full(params, &audio_data[..])?;

        let segments = state.full_n_segments()?;
        let mut out = Vec::with_capacity(segments as _);

        for segment in 0..segments {
            out.push(Segment {
                content: state.full_get_segment_text(segment)?,
                start: state.full_get_segment_t0(segment)?,
                end: state.full_get_segment_t1(segment)?,
            })
        }

        Ok(out)
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub start: i64,
    pub end: i64,
    pub content: String,
}
