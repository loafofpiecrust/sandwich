use crate::grammar;
use crate::sawtooth::SawtoothWave;
use anyhow;
use itertools::Itertools;
use rodio::{self, Sink};
use std::{thread, time::Duration};

const VOLUME: f32 = 0.4;

pub fn play_sound(frequencies: (u32, u32), duration: Duration) -> anyhow::Result<()> {
    let device = rodio::default_output_device().expect("Failed to setup audio output");
    let sink1 = Sink::new(&device);
    let sink2 = Sink::new(&device);

    sink1.set_volume(VOLUME);
    sink2.set_volume(VOLUME);
    // Play two sine waves at once for dual-tone effect.
    sink1.append(SawtoothWave::new(frequencies.0));
    sink2.append(rodio::source::SineWave::new(frequencies.1));

    thread::sleep(duration);

    sink1.stop();
    sink2.stop();

    Ok(())
}

pub fn play_phrase(phrase: &str, shift: f64) -> anyhow::Result<()> {
    for w in phrase.split(" ") {
        play_word(w, shift)?;
        // crate::wait_randomly(100);
    }
    Ok(())
}

fn mult_freq(f: u32, shift: f64) -> u32 {
    (f as f64 * shift) as u32
}

/// Assumes strict CV syllable structure for now.
pub fn play_word(word: &str, shift: f64) -> anyhow::Result<()> {
    for (a, b) in word.chars().tuples() {
        play_sound(
            (
                mult_freq(consonant_sound(a), shift),
                mult_freq(vowel_sound(b), shift),
            ),
            Duration::from_millis(200),
        )?;
    }
    Ok(())
}

const CONSONANT_FREQS: &[u32] = &[460, 510, 560, 620, 697, 770, 852, 941, 1040];
const VOWEL_FREQS: &[u32] = &[1209, 1336, 1477, 1600, 1720];
pub fn consonant_sound(letter: char) -> u32 {
    CONSONANT_FREQS[grammar::CONSONANTS
        .chars()
        .position(|x| x == letter)
        .unwrap()]
}

pub fn vowel_sound(letter: char) -> u32 {
    VOWEL_FREQS[grammar::VOWELS.chars().position(|x| x == letter).unwrap()]
}
