# Audio corpus attribution

`corpus_frames.bin` is derived from real human Mandarin recordings:

- **Source:** [hugolpz/audio-cmn](https://github.com/hugolpz/audio-cmn) (repackaging of
  Shtooka / BLM native-speaker recordings)
- **Speakers:** Chen Wang & Yue Tan (native Mandarin)
- **License:** CC-BY-SA — attribution + share-alike. Derivative audio/frames must
  retain this attribution and remain under a compatible share-alike license.

Files in `mandarin_seed/` (not deployed): isolated syllables `cmn-{pinyin}{tone}.mp3`
from `64k/syllabs/` and caregiver words 妈妈/爸爸 from `64k/hsk/`, converted to
48 kHz mono WAV, then transformed to 64-band mel frames by `build_corpus.mjs`.
