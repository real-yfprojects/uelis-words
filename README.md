# uelis-words

## Install

- `whisper-rs` (deps)
- model from [here](https://huggingface.co/ggerganov/whisper.cpp), saved to `whisper.model` in root dir
- don't use `large` (?), use `large-v2` (install with `make large-v2` in `whisper.cpp`)
- directly use whisper? (more options, e.g. as vtt (detailed word timestamps), initial prompt to capture special stuff (e.g. erms, "special" words))

## Workflow

1. login with credentials with a `POST` request at `https://video.ethz.ch/lectures/d-infk/2023/autumn/252-0025-01L.series-login.json`
2. fetch all lectures at `https://video.ethz.ch/lectures/d-infk/2023/autumn/252-0025-01L.series-metadata.json`
3. get more info for each new lecture at `https://video.ethz.ch/lectures/d-infk/2023/autumn/252-0025-01L/{}.series-metadata.json`
4. download the lowest resolution `.mp4` (`.mp3` quality is the same)
5. convert to `.mp3` with `ffmpeg -i path/to/input.mp4 -ac 1 -ar 16000 -acodec pcm_s16le -f wav path/to/out.wav -y` (to 16kHz mono wav)
6. transcribe using `large-v2` from `whisper.cpp`, `segmented` mode, language set to `de`
7. output as `.json`
