#!/bin/bash
set -exu pipefail

for filename in videos/*.mp4; do
    whisper "$filename" --model large-v2 --model_dir models/ --output_dir outputs/ --output_format vtt --language de
done