use crate::grammar;
use anyhow;
use itertools::Itertools;
use rodio::{self, Sink};
use std::{f32, thread, time::Duration};

pub fn play_sound(frequencies: (f32, f32), duration: Duration) -> anyhow::Result<()> {
    let device = rodio::default_output_device().unwrap();
    let sink1 = Sink::new(&device);
    let sink2 = Sink::new(&device);

    // Play two sine waves at once for dual-tone effect.
    sink1.append(rodio::source::SineWave::new(frequencies.0 as u32));
    sink2.append(rodio::source::SineWave::new(frequencies.1 as u32));

    thread::sleep(duration);

    sink1.stop();
    sink2.stop();

    Ok(())
}

pub fn play_phrase(phrase: &str) -> anyhow::Result<()> {
    for w in phrase.split(" ") {
        play_word(w)?;
        thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

/// Assumes strict CV syllable structure for now.
pub fn play_word(word: &str) -> anyhow::Result<()> {
    for (a, b) in word.chars().tuples() {
        play_sound(
            (consonant_sound(a), vowel_sound(b)),
            Duration::from_millis(200),
        )?;
    }
    Ok(())
}

pub fn consonant_sound(letter: char) -> f32 {
    1200.0
        + grammar::CONSONANTS
            .chars()
            .position(|x| x == letter)
            .unwrap() as f32
            * 130.0
}

pub fn vowel_sound(letter: char) -> f32 {
    690.0 + grammar::VOWELS.chars().position(|x| x == letter).unwrap() as f32 * 80.0
}
