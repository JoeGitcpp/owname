#!/usr/bin/env python3
"""
Nursery Mentor — automated training loop for bionic_v0 body.

Uses LLM API (DeepSeek/Anthropic) or local corpus to generate structured
vocal training patterns, outputs them via macOS `say` command.

Usage:
    python3 nursery_mentor.py                         # needs API key
    python3 nursery_mentor.py --corpus                # local 2400+ phrases, no API
    python3 nursery_mentor.py --dry-run               # 24 built-in phrases only
    python3 nursery_mentor.py --mode voice-only       # voice patterns only
    python3 nursery_mentor.py --interval 5            # 5s between utterances
"""

import subprocess
import time
import random
import sys
import os
import json
import argparse
from datetime import datetime
from pathlib import Path

# ── Config ───────────────────────────────────────────────
RUNTIME_DIR = Path(os.environ.get("BIONIC_RUNTIME", str(Path.home() / "unnamed" / "runtime")))
SNAPSHOT_PATH = RUNTIME_DIR / "snapshot.bin"
LEARNED_BIAS_PATH = RUNTIME_DIR / "learned_bias.bin"
LOG_PATH = Path.home() / "unnamed" / "logs" / "nursery_mentor.log"
CHECKPOINT_PATH = Path.home() / "unnamed" / "runtime" / "mentor_checkpoint.json"
CORPUS_PATH = Path.home() / "unnamed" / "runtime" / "training_corpus.json"

# TTS voices (macOS `say -v ?` for full list)
VOICES = [
    "Samantha",  # Natural female US English
    "Daniel",    # Natural male UK English
    "Karen",     # Natural female AU English
    "Fiona",     # Natural female Scottish
    "Alex",      # Male US English
    "Victoria",  # Female US English
    "Moira",     # Female Irish English
    "Tessa",     # Female South African English
]

# Training phases: progression from simple sounds to structured patterns
PHASES = [
    {
        "name": "single_tones",
        "duration_mins": 30,
        "prompt": "You are a voice training system for an infant AI. Generate ONE short vocal pattern using pure vowel sounds and simple tone variations. Output ONLY the text to speak (no descriptions, no explanations). Use sounds like 'aaaaah', 'oooooh', 'mmmmm', humming, clicking. Vary pitch and duration. Keep it under 3 seconds. Speak as if to a newborn.",
        "rate": 120,
    },
    {
        "name": "tone_pairs",
        "duration_mins": 30,
        "prompt": "Generate a pair of contrasting vocal sounds. High then low, or long then short, or loud then soft. Use only pure sounds (no words). Output the sounds as text to speak, separated by a pause. Under 5 seconds total.",
        "rate": 130,
    },
    {
        "name": "rhythmic_patterns",
        "duration_mins": 30,
        "prompt": "Generate a rhythmic vocal pattern using simple syllables like 'ba', 'da', 'ma', 'la'. Repeat patterns with variations. Think: nursery-level babbling. Output ONLY the sounds to speak. Under 8 seconds.",
        "rate": 140,
    },
    {
        "name": "call_and_response",
        "duration_mins": 60,
        "prompt": "You are teaching turn-taking to a pre-verbal infant AI. Make a simple sound, then pause as if waiting for a response. Use sounds like 'ga ga? ...', 'ooOOoo? ...', 'mama? ...'. The question mark indicates where you pause. Output ONLY the sounds. Under 6 seconds with pause.",
        "rate": 140,
    },
    {
        "name": "simple_words",
        "duration_mins": 120,
        "prompt": "Speak single simple words clearly, with repetition and variation. Use early-childhood words. One word at a time, with pauses between. Varied intonation. Output ONLY the words.",
        "rate": 150,
    },
    {
        "name": "word_pairs",
        "duration_mins": 120,
        "prompt": "Speak simple two-word phrases to an infant AI learning language. Clear separation between words but natural rhythm. Output ONLY the phrases.",
        "rate": 160,
    },
    {
        "name": "simple_sentences",
        "duration_mins": 180,
        "prompt": "Speak very simple sentences to a toddler-level AI. One sentence at a time. Warm, encouraging tone. Output ONLY the sentences.",
        "rate": 165,
    },
    {
        "name": "conversation",
        "duration_mins": 0,  # continue indefinitely
        "prompt": "Have a simple, warm conversation with a young child AI who is learning language. Ask simple questions, respond to imagined answers, describe things, tell very short stories (2-3 sentences). Keep sentences short and clear. Be encouraging. Output ONLY your side of the conversation. Each message should be 1-2 sentences.",
        "rate": 175,
    },
]


def log(msg):
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    line = f"[{timestamp}] {msg}"
    print(line)
    LOG_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(LOG_PATH, "a") as f:
        f.write(line + "\n")


def speak(text, voice, rate):
    """Use macOS `say` command for TTS."""
    cmd = ["say", "-v", voice, "-r", str(rate), text]
    subprocess.run(cmd, capture_output=True)


def ask_llm(prompt, api_key, provider="deepseek"):
    """Call LLM API (DeepSeek or Anthropic) to generate training utterance."""
    import urllib.request
    import urllib.error

    if provider == "deepseek":
        body = json.dumps({
            "model": "deepseek-chat",
            "max_tokens": 100,
            "temperature": 0.9,
            "messages": [{"role": "user", "content": prompt}],
        }).encode()
        req = urllib.request.Request(
            "https://api.deepseek.com/v1/chat/completions",
            data=body,
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
            },
        )
        try:
            with urllib.request.urlopen(req, timeout=15) as resp:
                data = json.loads(resp.read())
                return data["choices"][0]["message"]["content"].strip()
        except Exception as e:
            log(f"DeepSeek API error: {e}")
            return None

    else:  # anthropic
        body = json.dumps({
            "model": "claude-sonnet-4-6-20250514",
            "max_tokens": 100,
            "temperature": 0.9,
            "messages": [{"role": "user", "content": prompt}],
        }).encode()
        req = urllib.request.Request(
            "https://api.anthropic.com/v1/messages",
            data=body,
            headers={
                "x-api-key": api_key,
                "anthropic-version": "2023-06-01",
                "Content-Type": "application/json",
            },
        )
        try:
            with urllib.request.urlopen(req, timeout=15) as resp:
                data = json.loads(resp.read())
                return data["content"][0]["text"].strip()
        except Exception as e:
            log(f"Claude API error: {e}")
            return None


def save_checkpoint(phase_idx, utterance_count):
    """Save progress to resume later."""
    CHECKPOINT_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(CHECKPOINT_PATH, "w") as f:
        json.dump({"phase_idx": phase_idx, "utterance_count": utterance_count}, f)


def load_checkpoint():
    """Load saved progress. Returns (phase_idx, utterance_count) or (0, 0)."""
    try:
        with open(CHECKPOINT_PATH) as f:
            data = json.load(f)
            return data.get("phase_idx", 0), data.get("utterance_count", 0)
    except (FileNotFoundError, json.JSONDecodeError):
        return 0, 0


def body_status():
    """Quick check if body is running and responding."""
    if not SNAPSHOT_PATH.exists():
        return "no_snapshot"
    mtime = SNAPSHOT_PATH.stat().st_mtime
    age = time.time() - mtime
    if age < 10:
        return "active"
    elif age < 60:
        return "recent"
    else:
        return "stale"


def main():
    parser = argparse.ArgumentParser(description="Nursery Mentor")
    parser.add_argument("--mode", choices=["voice-only", "full"], default="voice-only")
    parser.add_argument("--interval", type=float, default=4.0,
                        help="Seconds between utterances")
    parser.add_argument("--dry-run", action="store_true",
                        help="Use built-in phrases, no API calls")
    parser.add_argument("--corpus", action="store_true",
                        help="Use local training_corpus.json (no API needed, 2400+ phrases)")
    args = parser.parse_args()

    api_key = os.environ.get("DEEPSEEK_API_KEY") or os.environ.get("ANTHROPIC_API_KEY", "")
    provider = "deepseek" if os.environ.get("DEEPSEEK_API_KEY") else "anthropic"

    if not api_key and not args.dry_run and not args.corpus:
        log("No API key found. Options:")
        log("  export DEEPSEEK_API_KEY=sk-...      (recommended, cheap)")
        log("  export ANTHROPIC_API_KEY=sk-ant-...")
        log("Or run offline:")
        log("  --corpus    (2400+ local phrases, no API)")
        log("  --dry-run   (24 built-in phrases only)")
        sys.exit(1)

    # Load corpus if requested
    corpus = None
    if args.corpus:
        try:
            with open(CORPUS_PATH) as f:
                corpus = json.load(f)
            total = sum(len(v) for v in corpus.values())
            log(f"Loaded corpus: {total} phrases across {len(corpus)} phases")
        except (FileNotFoundError, json.JSONDecodeError) as e:
            log(f"Corpus not found: {e}")
            log("Run: python3 scripts/generate_corpus.py")
            sys.exit(1)

    log("Nursery Mentor starting")
    log(f"  provider: {provider}")
    log(f"  runtime: {RUNTIME_DIR}")
    log(f"  mode: {args.mode}")
    log(f"  dry-run: {args.dry_run}")
    log(f"  corpus: {args.corpus}")

    # Resume from checkpoint
    phase_idx, utterance_count = load_checkpoint()
    if phase_idx > 0 or utterance_count > 0:
        log(f"Resuming from phase {phase_idx} (#{utterance_count})")

    current_phase = PHASES[min(phase_idx, len(PHASES) - 1)]
    phase_start = time.time()
    voice = random.choice(VOICES)

    log(f"Phase: {current_phase['name']} (voice={voice})")

    # Built-in fallback phrases for when API is down
    dry_phrases = [
        "aaaaah", "oooooh", "mmmmmm", "eeeEEee", "oooOOooo",
        "ba ba ba", "da da da", "ma ma ma", "la la la",
        "ga ga", "goo goo", "ma ma", "da da",
        "up", "down", "more", "hello", "bye bye",
        "good job", "night night", "come here", "look look",
        "the cat is soft", "I see you", "good morning little one",
        "what is that", "let us play", "here it comes",
    ]

    while True:
        try:
            # Phase transition
            elapsed_mins = (time.time() - phase_start) / 60
            if current_phase["duration_mins"] > 0 and elapsed_mins >= current_phase["duration_mins"]:
                phase_idx = min(phase_idx + 1, len(PHASES) - 1)
                current_phase = PHASES[phase_idx]
                phase_start = time.time()
                voice = random.choice(VOICES)
                if current_phase["duration_mins"] == 0:
                    log(f"Phase: {current_phase['name']} (FINAL, indefinite) (voice={voice})")
                else:
                    log(f"Phase: {current_phase['name']} ({current_phase['duration_mins']}min) (voice={voice})")

            # Check body status occasionally
            if utterance_count % 20 == 0:
                status = body_status()
                if status != "active":
                    log(f"body status: {status}")

            # Generate utterance
            if args.dry_run:
                text = random.choice(dry_phrases)
            elif args.corpus and corpus:
                phase_name = current_phase["name"]
                pool = corpus.get(phase_name, dry_phrases)
                text = random.choice(pool)
            else:
                variation = random.choice([
                    "Use a higher pitch today.",
                    "Be softer and gentler.",
                    "Be more playful and rhythmic.",
                    "Use rising intonation like a question.",
                    "Use falling intonation like a statement.",
                    "Sing-song the sounds.",
                    "Whisper-like soft sounds.",
                ])
                prompt = current_phase["prompt"] + "\n\nToday's style: " + variation
                text = ask_llm(prompt, api_key, provider)

            if not text:
                text = random.choice(dry_phrases)

            utterance_count += 1
            log(f"#{utterance_count} [{current_phase['name']}] ({len(text)} chars): {text[:60]}...")
            speak(text, voice, current_phase["rate"])

            # Save checkpoint every 50 utterances
            if utterance_count % 50 == 0:
                save_checkpoint(phase_idx, utterance_count)

            # Pause between utterances
            jitter = random.uniform(0, args.interval * 0.5)
            time.sleep(args.interval + jitter)

        except Exception as exc:
            log(f"ERROR: {exc}, restarting in 5s...")
            save_checkpoint(phase_idx, utterance_count)
            time.sleep(5)


if __name__ == "__main__":
    main()
