#!/bin/bash
set -exu pipefall

# Setup
mkdir -p outputs

# ffmpeg must be installed
# install whisper, torch reqs?
pip install git+https://github.com/openai/whisper.git

# transcribe each .mp3 using whisper as vtt, about 10GB VRAM for large-v2 (how much ram?)
bash transcribe.sh