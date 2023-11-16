#!/bin/bash
set -exu pipefall

# Setup
mkdir -p outputs
mkdir -p wavs

# convert each .mp4 to .wav
# for filename in videos/*.mp4; do
#     ffmpeg -i "$filename" -acodec pcm_s16le -ac 1 -ar 16000 "wavs/$(basename "$filename" .mp4).wav"
# done

# ffmpeg must be installed
# install whisper, torch reqs?

sudo apt-get update
sudo apt-get -y install make build-essential git

git clone https://github.com/ggerganov/whisper.cpp.git

cd whisper.cpp
make large-v2
make quantize
./quantize models/ggml-large-v2.bin models/ggml-large-v2q.bin 2
cd ..