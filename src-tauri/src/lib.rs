#![allow(clippy::type_complexity)]
#![allow(clippy::blocks_in_if_conditions)]

use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashSet, num::NonZeroU8, sync::Mutex}; // Add serde imports

include!(concat!(env!("OUT_DIR"), "/dictionary.rs"));

mod solver;
use solver::{GuessResult, Solver};

// State struct to hold our Solver instance
struct AppState {
    guesser: Mutex<Solver>,
}

// Define a serializable version of Guess for the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiGuess {
    pub word: String,
    pub mask: [ApiCorrectness; 5],
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn play(state: tauri::State<AppState>, history: Vec<ApiGuess>) -> Result<Vec<GuessResult>, String> {
    let dictionary: HashSet<&'static str> =
        HashSet::from_iter(DICTIONARY.iter().copied().map(|(word, _)| word));

    if !history.is_empty() && !dictionary.contains(history[0].word.as_str()) {
        return Err(format!("Word not in dictionary: {}", history[0].word));
    }

    // Convert ApiGuess to internal Guess
    let internal_history: Vec<Guess> = history
        .into_iter()
        .map(|api_guess| Guess {
            word: Cow::Owned(api_guess.word),
            mask: api_guess.mask.map(|c| c.into()),
        })
        .collect();

    let mut guesser = state.guesser.lock().unwrap();
    let results = guesser.guess(&internal_history);

    Ok(results)
}

// Reset function to replace the solver with a new instance
#[tauri::command]
fn reset(state: tauri::State<AppState>) {
    let mut guesser = state.guesser.lock().unwrap();
    *guesser = Solver::new(false);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState {
        guesser: Mutex::new(Solver::new(false)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![play, reset])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiCorrectness {
    /// Green
    Correct,
    /// Yellow
    Misplaced,
    /// Gray
    Wrong,
}

// Add conversion between ApiCorrectness and Correctness
impl From<ApiCorrectness> for Correctness {
    fn from(api: ApiCorrectness) -> Self {
        match api {
            ApiCorrectness::Correct => Correctness::Correct,
            ApiCorrectness::Misplaced => Correctness::Misplaced,
            ApiCorrectness::Wrong => Correctness::Wrong,
        }
    }
}

impl From<Correctness> for ApiCorrectness {
    fn from(c: Correctness) -> Self {
        match c {
            Correctness::Correct => ApiCorrectness::Correct,
            Correctness::Misplaced => ApiCorrectness::Misplaced,
            Correctness::Wrong => ApiCorrectness::Wrong,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Correctness {
    /// Green
    Correct,
    /// Yellow
    Misplaced,
    /// Gray
    Wrong,
}

impl Correctness {
    fn is_misplaced(letter: u8, answer: &str, used: &mut [bool; 5]) -> bool {
        answer.bytes().enumerate().any(|(i, a)| {
            if a == letter && !used[i] {
                used[i] = true;
                return true;
            }
            false
        })
    }

    pub fn compute(answer: &str, guess: &str) -> [Self; 5] {
        assert_eq!(answer.len(), 5);
        assert_eq!(guess.len(), 5);
        let mut c = [Correctness::Wrong; 5];
        let answer_bytes = answer.as_bytes();
        let guess_bytes = guess.as_bytes();
        // Array indexed by lowercase ascii letters
        let mut misplaced = [0u8; (b'z' - b'a' + 1) as usize];

        // Find all correct letters
        for ((&answer, &guess), c) in answer_bytes.iter().zip(guess_bytes).zip(c.iter_mut()) {
            if answer == guess {
                *c = Correctness::Correct
            } else {
                // If the letter does not match, count it as misplaced
                misplaced[(answer - b'a') as usize] += 1;
            }
        }
        // Check all of the non matching letters if they are misplaced
        for (&guess, c) in guess_bytes.iter().zip(c.iter_mut()) {
            // If the letter was guessed wrong and the same letter was counted as misplaced
            if *c == Correctness::Wrong && misplaced[(guess - b'a') as usize] > 0 {
                *c = Correctness::Misplaced;
                misplaced[(guess - b'a') as usize] -= 1;
            }
        }

        c
    }
}

pub const MAX_MASK_ENUM: usize = 3 * 3 * 3 * 3 * 3;

/// A wrapper type for `[Correctness; 5]` packed into a single byte with a niche.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
// The NonZeroU8 here lets the compiler know that we're not using the value `0`, and that `0` can
// therefore be used to represent `None` for `Option<PackedCorrectness>`.
struct PackedCorrectness(NonZeroU8);

impl From<[Correctness; 5]> for PackedCorrectness {
    fn from(c: [Correctness; 5]) -> Self {
        let packed = c.iter().fold(0, |acc, c| {
            acc * 3
                + match c {
                    Correctness::Correct => 0,
                    Correctness::Misplaced => 1,
                    Correctness::Wrong => 2,
                }
        });
        Self(NonZeroU8::new(packed + 1).unwrap())
    }
}

impl From<PackedCorrectness> for u8 {
    fn from(this: PackedCorrectness) -> Self {
        this.0.get() - 1
    }
}

pub struct Guess<'a> {
    pub word: Cow<'a, str>,
    pub mask: [Correctness; 5],
}

impl Guess<'_> {
    pub fn matches(&self, word: &str) -> bool {
        // Check if the guess would be possible to observe when `word` is the correct answer.
        // This is equivalent to
        //     Correctness::compute(word, &self.word) == self.mask
        // without _necessarily_ computing the full mask for the tested word
        assert_eq!(word.len(), 5);
        assert_eq!(self.word.len(), 5);
        let mut used = [false; 5];

        // Check Correct letters
        for (i, (a, g)) in word.bytes().zip(self.word.bytes()).enumerate() {
            if a == g {
                if self.mask[i] != Correctness::Correct {
                    return false;
                }
                used[i] = true;
            } else if self.mask[i] == Correctness::Correct {
                return false;
            }
        }

        // Check Misplaced letters
        for (g, e) in self.word.bytes().zip(self.mask.iter()) {
            if *e == Correctness::Correct {
                continue;
            }
            if Correctness::is_misplaced(g, word, &mut used) != (*e == Correctness::Misplaced) {
                return false;
            }
        }

        // The rest will be all correctly Wrong letters
        true
    }
}
