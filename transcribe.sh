#!/bin/bash
set -exu pipefail

# transcribe each .wav using whisper.cpp as json
for filename in wavs/*.wav; do
    ./whisper.cpp/main -l de -m whisper.model --output-json-full -pp -of "outputs/$(basename "$filename" .wav)" -f "$filename"
done