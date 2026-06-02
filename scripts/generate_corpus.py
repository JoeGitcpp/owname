#!/usr/bin/env python3
"""
Generate a rich local training corpus for nursery_mentor.
Run once to create runtime/training_corpus.json, then use --corpus mode.
No API key needed.
"""

import json
import random
from pathlib import Path

OUTPUT = Path.home() / "unnamed" / "runtime" / "training_corpus.json"


def gen_single_tones(n=400):
    """Pure vowel sounds, humming, clicking, pitch/breath variations."""
    vowels = ["a", "e", "i", "o", "u"]
    patterns = []

    # Templates for varied output
    for _ in range(n):
        t = random.choice([
            # Basic sustained vowels
            lambda: "".join(random.choice(vowels) * random.randint(2, 8)).capitalize() + "~",
            lambda: random.choice(vowels) * random.randint(2, 6) + "h" + "..." * random.randint(0, 2),
            # Rising/falling
            lambda: random.choice(vowels) * random.randint(1, 3) + random.choice(vowels).upper() * random.randint(2, 5),
            lambda: random.choice(vowels).upper() * random.randint(2, 4) + random.choice(vowels) * random.randint(2, 4),
            # Mixed vowel chains
            lambda: "-".join([v * random.randint(1, 4) for v in random.sample(vowels, random.randint(2, 4))]),
            # Humming
            lambda: "M" + "m" * random.randint(2, 8) + random.choice(["~", "...", "?", "!"]),
            lambda: "H" + "m" * random.randint(2, 6) + random.choice(["", "mm...", "-hmm"]),
            # Breathy
            lambda: "H" + "h" * random.randint(1, 4) + random.choice(["ah", "uh", "oh", "eh"]) + "..." * random.randint(0, 2),
            # Click/tsk variants
            lambda: random.choice(["tsk-tsk", "cluck-cluck", "pop", "click"]) + "-" + random.choice(vowels) * random.randint(2, 4) + "!",
            # Sigh variations
            lambda: random.choice(["Sigh", "Hush", "Shh", "Sss"]) + "... " + random.choice(["mmmm", "aaah", "oooh"]) + "...",
            # Questioning tones
            lambda: random.choice(vowels) * random.randint(2, 5) + "? " + random.choice(vowels) * random.randint(2, 4) + "?",
            # Playful
            lambda: random.choice(["Hee-hee", "Ha-ha", "Ho-ho", "Hoo-hoo"]) + "! " + random.choice(vowels) * random.randint(2, 4) + "...",
            # Soft murmurs
            lambda: random.choice(["mmm", "nnn", "ngg"]) + "-" + random.choice(vowels) * random.randint(2, 4) + "...",
            # Long glide
            lambda: random.choice(vowels) * 2 + "aa" + random.choice([v for v in vowels if v != "a"]) * random.randint(2, 4) + "h...",
        ])
        text = t().strip()
        if 2 < len(text) < 60:
            patterns.append(text)

    return patterns


def gen_tone_pairs(n=350):
    """Contrasting pairs: high/low, long/short, loud/soft."""
    patterns = []
    highs = ["Eee", "Eeee", "Ii", "Iiii", "Ee-ee", "Mmm?", "AH?", "Wheee"]
    lows = ["oooh", "ooooh", "ahhh", "awww", "mmm.", "uhhh", "ohh.", "umm."]
    actions = [
        ("rising squeak", "low rumble"),
        ("bright chirp", "deep hum"),
        ("high question", "low answer"),
        ("sharp call", "soft response"),
        ("long held note", "short staccato"),
        ("loud burst", "whispered reply"),
        ("ascending", "descending"),
        ("front-of-mouth", "back-of-throat"),
    ]

    for _ in range(n):
        high = random.choice(highs)
        low = random.choice(lows)
        sep = random.choice([
            f" ... (pause) ... ",
            f" [pause] ",
            f", then softly: ",
            f" → ",
            f" ... {random.choice(['','... '])}",
        ])
        desc = random.choice(actions)
        texts = [
            f"{high}!{sep}{low}...",
            f"({desc[0]}): {high} ... ({desc[1]}): {low}",
            f"{high}? {sep}{low}.",
            f"High: {high}  /  Low: {low}",
            f"{high}{random.choice('!?')} (pause) {low}...",
            f"({high}) ...then... ({low})",
            f"{high}~ ... {low}~",
        ]
        text = random.choice(texts)
        if len(text) < 80:
            patterns.append(text)

    return patterns


def gen_rhythmic_patterns(n=350):
    """Syllable babbling with rhythm."""
    syllables = ["ba", "da", "ma", "la", "ga", "goo", "boo", "pa", "ta", "na", "ka", "wa", "ya", "doo", "moo"]
    patterns = []

    for _ in range(n):
        syl = random.choice(syllables)
        count = random.randint(2, 5)
        repeats = " ".join([syl] * count)
        variants = [
            f"{repeats}...",
            f"{repeats}! {random.choice(syllables)} {random.choice(syllables)}...",
            f"{syl} {syl}... {random.choice(syllables)} {random.choice(syllables)} {random.choice(syllables)}...",
            f"{repeats} (click) {random.choice(syllables)} {random.choice(syllables)}",
            f"({syl}) ({syl} {syl})... ({random.choice(syllables)})",
            f"{syl}-{syl}-{syl}... {random.choice(syllables)}{random.choice(syllables)}{random.choice(syllables)}!",
            f"{repeats}... {' '.join([random.choice(syllables) for _ in range(2)])}...",
        ]
        text = random.choice(variants)
        if len(text) < 60:
            patterns.append(text)

    return patterns


def gen_call_and_response(n=300):
    """Turn-taking sounds with question-like endings."""
    sounds = ["ga ga", "goo goo", "ma ma", "da da", "ba ba", "oooOOoo", "eeeEEee",
              "mmmm", "aaaaah", "oooooh", "la la", "boo boo", "moo moo", "wa wa"]
    questions = [
        "...", "?", "? ...", "? (waiting)...", "...?", "... (pause)...",
        "? *pause*", "... *listens*...", "? ...", "? (expectant pause)...",
    ]

    patterns = []
    for _ in range(n):
        sound = random.choice(sounds)
        q = random.choice(questions)
        texts = [
            f"{sound}{q}",
            f"{sound}{q} {random.choice(sounds)}...",
            f"({sound}){q}",
            f"{sound}? ... {random.choice(sounds)}.",
            f"{sound}? (pause) {sound}.",
        ]
        text = random.choice(texts)
        if len(text) < 50:
            patterns.append(text)

    return patterns


def gen_simple_words(n=300):
    """Single early-childhood words with repetition."""
    words = [
        "up", "down", "more", "no", "yes", "hello", "bye", "bye bye",
        "mama", "dada", "ball", "dog", "cat", "big", "small", "hot",
        "cold", "good", "bad", "go", "stop", "come", "here", "there",
        "look", "see", "eat", "drink", "sleep", "night night", "good job",
        "uh oh", "wow", "yay", "open", "close", "in", "out", "on", "off",
        "one", "two", "red", "blue", "car", "book", "bird", "fish",
        "milk", "water", "please", "thank you", "hi", "hey", "gentle",
        "soft", "loud", "fast", "slow", "happy", "sad", "pretty",
        "funny", "nice", "yummy", "again", "all done", "peekaboo",
    ]
    patterns = []
    for _ in range(n):
        w = random.choice(words)
        repeat = random.randint(1, 3)
        texts = [
            f"{w}.{' ' + w + '.' if repeat > 1 else ''}",
            f"{w}... {w}!",
            f"{w}. (pause) {w}?",
            f"{w} {w} {w}..." if repeat > 2 else f"{w} {w}...",
            f"{w}! {w}...",
            f"({w}) ... {w}.",
        ]
        text = random.choice(texts)
        if len(text) < 40:
            patterns.append(text)

    return patterns


def gen_word_pairs(n=300):
    """Simple two-word phrases."""
    pairs = [
        ("red", "ball"), ("big", "dog"), ("more", "food"), ("go", "up"),
        ("come", "here"), ("good", "job"), ("night", "night"), ("bye", "bye"),
        ("all", "done"), ("uh", "oh"), ("thank", "you"), ("let's", "go"),
        ("little", "one"), ("sweet", "dreams"), ("see", "you"), ("look", "there"),
        ("what's", "that"), ("my", "turn"), ("your", "turn"), ("let's", "play"),
        ("nice", "job"), ("well", "done"), ("good", "morning"), ("hi", "there"),
        ("big", "hug"), ("pretty", "bird"), ("blue", "sky"), ("warm", "sun"),
        ("soft", "cat"), ("fast", "car"), ("yummy", "milk"), ("time", "sleep"),
        ("read", "book"), ("sing", "song"), ("play", "ball"), ("eat", "food"),
        ("drink", "water"), ("wash", "hands"), ("brush", "hair"), ("put", "on"),
        ("take", "off"), ("stand", "up"), ("sit", "down"), ("turn", "around"),
        ("clap", "hands"), ("wave", "bye"), ("blow", "kiss"), ("give", "hug"),
    ]
    patterns = []
    for _ in range(n):
        a, b = random.choice(pairs)
        texts = [
            f"{a} {b}.",
            f"{a} {b}... {a} {b}.",
            f"({a}) ... {b}!",
            f"{a} {b}? {a} {b}!",
            f"{a}... {a} {b}.",
        ]
        text = random.choice(texts)
        if len(text) < 50:
            patterns.append(text)

    return patterns


def gen_simple_sentences(n=400):
    """Short toddler-level sentences."""
    templates = [
        "The {noun} is {adj}.",
        "I see {noun}.",
        "That is {noun}.",
        "Come here {name}.",
        "Good morning {name}!",
        "What is {noun}?",
        "Let us {verb} together!",
        "Where is {noun}?",
        "I like {noun}.",
        "Can you {verb}?",
        "Time to {verb}.",
        "Let's {verb} {noun}.",
        "We are {verb}ing.",
        "Do you want {noun}?",
        "Show me {noun}.",
        "Give me {noun}.",
        "Point to {noun}.",
        "Say {word}.",
        "Listen... {sound}.",
        "Here it comes!",
        "Ready, set, {verb}!",
        "That's {adj}!",
        "You did it!",
        "Try again!",
        "One more time!",
        "All done now.",
        "It's {noun} time.",
        "Look at {noun}.",
        "What color?",
        "How many?",
    ]
    nouns = ["the cat", "a ball", "the dog", "a bird", "the sun", "a car",
             "the book", "a fish", "the sky", "a tree", "the moon", "a star",
             "baby", "the light", "a sound", "the door", "a flower"]
    adjs = ["soft", "big", "small", "red", "blue", "pretty", "funny", "nice",
            "warm", "bright", "loud", "quiet", "fast", "slow", "happy", "good"]
    verbs = ["play", "sing", "dance", "run", "jump", "clap", "eat", "sleep",
             "read", "go", "stop", "look", "listen", "try", "smile", "laugh"]
    names = ["little one", "baby", "sweetie", "darling", "buddy"]
    words = ["hello", "bye", "mama", "dada", "ball", "dog", "cat", "up", "down", "more"]
    sounds = ["shh", "beep beep", "moo", "woof", "meow", "quack", "tick tock",
              "ding dong", "boom", "splash"]

    patterns = []
    for _ in range(n):
        tmpl = random.choice(templates)
        try:
            text = tmpl.format(
                noun=random.choice(nouns),
                adj=random.choice(adjs),
                verb=random.choice(verbs),
                name=random.choice(names),
                word=random.choice(words),
                sound=random.choice(sounds),
            )
            if len(text) < 80:
                patterns.append(text)
        except KeyError:
            continue

    return patterns


def main():
    random.seed(42)

    corpus = {
        "single_tones": gen_single_tones(400),
        "tone_pairs": gen_tone_pairs(350),
        "rhythmic_patterns": gen_rhythmic_patterns(350),
        "call_and_response": gen_call_and_response(300),
        "simple_words": gen_simple_words(300),
        "word_pairs": gen_word_pairs(300),
        "simple_sentences": gen_simple_sentences(400),
    }

    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    with open(OUTPUT, "w") as f:
        json.dump(corpus, f, indent=2, ensure_ascii=False)

    total = sum(len(v) for v in corpus.values())
    print(f"Generated {total} phrases across {len(corpus)} phases")
    for phase, phrases in corpus.items():
        print(f"  {phase}: {len(phrases)} phrases")
    print(f"Saved to {OUTPUT}")


if __name__ == "__main__":
    main()
