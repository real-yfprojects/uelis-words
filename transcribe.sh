#!/bin/bash
set -exu pipefail

for filename in videos/*.mp4; do
    ffmpeg -i "$filename" -acodec pcm_s16le -ac 1 -ar 16000 "wavs/$(basename "$filename" .mp4).wav"
done

# transcribe each .wav using whisper.cpp as json
for filename in wavs/*.wav; do
    ./whisper.cpp/main -l de -m whisper.cpp/models/ggml-large-v2q.bin --output-json-full -pp -of "outputs/$(basename "$filename" .wav)" -f "$filename"
    echo "$filename" >> transcribed.txt
    break;
done