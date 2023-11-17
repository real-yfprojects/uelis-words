#!/bin/bash
set -exu pipefail

# transcribe each .wav using whisper.cpp as json
for filename in wavs/*.wav; do
    if grep -Fxq "$FILENAME" transcribed.txt
    then
        ./whisper.cpp/main -l de -m whisper.cpp/models/ggml-large-v2q.bin --output-json-full -pp -of "outputs/$(basename "$filename" .wav)" -f "$filename"
        echo "$filename" >> transcribed.txt
        break;
    fi
done