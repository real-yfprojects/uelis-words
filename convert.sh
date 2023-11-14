#!/bin/bash
set -exu pipefall

# Setup
mkdir -p wavs
mkdir -p outputs

# ffmpeg must be installed
# install whisper, torch reqs?
pip install git+https://github.com/openai/whisper.git

# transcribe each .mp3 using whisper as vtt, about 10GB VRAM for large-v2 (how much ram?)
for filename in videos/*.mp4; do
    whisper "$filename" --model large-v2 --model_dir models/ --output_dir outputs/ --output_format vtt --language de
done

# remove video and wav dir
# rm -rf /wavs
# rm -rf /videos