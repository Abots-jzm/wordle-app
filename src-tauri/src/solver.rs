use crate::{Correctness, Guess, PackedCorrectness, DICTIONARY, MAX_MASK_ENUM};
use once_cell::sync::OnceCell;
use once_cell::unsync::OnceCell as UnSyncOnceCell;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cell::Cell;

const FIRST_GUESS: &str = "tares";
static INITIAL_SIGMOID: OnceCell<Vec<(&'static str, f64, usize)>> = OnceCell::new();

type Cache = [[Cell<Option<PackedCorrectness>>; DICTIONARY.len()]; DICTIONARY.len()];
thread_local! {
    static COMPUTES: UnSyncOnceCell<Box<Cache>> = Default::default();
}

pub struct Solver {
    remaining: Cow<'static, Vec<(&'static str, f64, usize)>>,
    entropy: Vec<f64>,
    hard_mode: bool,
    last_guess_idx: Option<usize>,
}

// New struct to return the guess results, with Serialize/Deserialize
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuessResult {
    pub word: String,
    pub score: f64,
}

impl Default for Solver {
    fn default() -> Self {
        Solver::new(false)
    }
}

impl Solver {
    pub fn new(hard_mode: bool) -> Self {
        let remaining = INITIAL_SIGMOID.get_or_init(|| {
            let sum: usize = DICTIONARY.iter().map(|(_, count)| count).sum();

            DICTIONARY
                .iter()
                .copied()
                .enumerate()
                .map(|(idx, (word, count))| (word, sigmoid(count as f64 / sum as f64), idx))
                .collect()
        });

        COMPUTES.with(|c| {
            c.get_or_init(|| {
                let c = &Cell::new(None::<PackedCorrectness>);
                assert_eq!(std::mem::size_of_val(c), 1);
                let c = c as *const _;
                let c = c as *const u8;
                assert_eq!(unsafe { *c }, 0);

                let mem = unsafe {
                    std::alloc::alloc_zeroed(
                        std::alloc::Layout::from_size_align(
                            std::mem::size_of::<Cache>(),
                            std::mem::align_of::<Cache>(),
                        )
                        .unwrap(),
                    )
                };

                unsafe { Box::from_raw(mem as *mut _) }
            });
        });

        Self {
            remaining: Cow::Borrowed(remaining),
            entropy: Vec::new(),
            last_guess_idx: None,
            hard_mode,
        }
    }

    fn trim(&mut self, mut cmp: impl FnMut(&str, usize) -> bool) {
        if matches!(self.remaining, Cow::Owned(_)) {
            self.remaining
                .to_mut()
                .retain(|&(word, _, word_idx)| cmp(word, word_idx));
        } else {
            self.remaining = Cow::Owned(
                self.remaining
                    .iter()
                    .filter(|(word, _, word_idx)| cmp(word, *word_idx))
                    .copied()
                    .collect(),
            );
        }
    }
}

fn est_steps_left(entropy: f64) -> f64 {
    (entropy * 3.870 + 3.679).ln()
}

const L: f64 = 1.0;
// How steep is the cut-off?
const K: f64 = 30000000.0;
// Where is the cut-off?
const X0: f64 = 0.00000497;

fn sigmoid(p: f64) -> f64 {
    L / (1.0 + (-K * (p - X0)).exp())
}

// This inline gives about a 13% speedup.
#[inline]
fn get_packed(
    row: &[Cell<Option<PackedCorrectness>>],
    guess: &str,
    answer: &str,
    answer_idx: usize,
) -> PackedCorrectness {
    let cell = &row[answer_idx];
    match cell.get() {
        Some(a) => a,
        None => {
            let correctness = PackedCorrectness::from(Correctness::compute(answer, guess));
            cell.set(Some(correctness));
            correctness
        }
    }
}

impl Solver {
    pub fn guess(&mut self, history: &[Guess]) -> Vec<GuessResult> {
        let score = history.len() as f64;

        if let Some(last) = history.last() {
            // We need to find the word index for the word the user actually used
            let word_to_find = &last.word;

            // Find the index of the word in DICTIONARY
            let word_idx = DICTIONARY
                .iter()
                .position(|(word, _)| *word == word_to_find)
                .unwrap_or_else(|| {
                    // If we can't find it, just use the last guess index as fallback
                    // (This shouldn't happen with proper validation)
                    self.last_guess_idx.unwrap_or(0)
                });

            // Set the last guess index to the actual word used
            self.last_guess_idx = Some(word_idx);

            // Now filter using the correct word and its correctness pattern
            let reference = PackedCorrectness::from(last.mask);
            COMPUTES.with(|c| {
                let row = &c.get().unwrap()[word_idx];
                self.trim(|word, word_idx| {
                    reference == get_packed(row, &last.word, word, word_idx)
                });
            });
        }

        if history.is_empty() {
            self.last_guess_idx = Some(
                self.remaining
                    .iter()
                    .find(|(word, _, _)| &**word == FIRST_GUESS)
                    .map(|&(_, _, idx)| idx)
                    .unwrap(),
            );
            return vec![GuessResult {
                word: FIRST_GUESS.to_string(),
                score: 0.0,
            }];
        } else if self.remaining.len() == 1 {
            let w = self.remaining.first().unwrap();
            self.last_guess_idx = Some(w.2);
            return vec![GuessResult {
                word: w.0.to_string(),
                score: 0.0,
            }];
        }
        assert!(!self.remaining.is_empty());

        let remaining_p: f64 = self.remaining.iter().map(|&(_, p, _)| p).sum();
        let remaining_entropy = -self
            .remaining
            .iter()
            .map(|&(_, p, _)| {
                let p = p / remaining_p;
                p * p.log2()
            })
            .sum::<f64>();
        self.entropy.push(remaining_entropy);

        let mut candidates = Vec::new();
        let mut i = 0;
        let stop = (self.remaining.len() / 3).max(20).min(self.remaining.len());
        let consider = if self.hard_mode {
            &*self.remaining
        } else {
            INITIAL_SIGMOID.get().unwrap()
        };
        for &(word, count, word_idx) in consider {
            let mut totals = [0.0f64; MAX_MASK_ENUM];

            let mut in_remaining = false;
            COMPUTES.with(|c| {
                let row = &c.get().unwrap()[word_idx];
                for (candidate, count, candidate_idx) in &*self.remaining {
                    in_remaining |= word_idx == *candidate_idx;
                    let idx = get_packed(row, word, candidate, *candidate_idx);
                    totals[usize::from(u8::from(idx))] += count;
                }
            });

            let sum: f64 = totals
                .into_iter()
                .filter(|t| *t != 0.0)
                .map(|p| {
                    let p_of_this_pattern = p as f64 / remaining_p as f64;
                    p_of_this_pattern * p_of_this_pattern.log2()
                })
                .sum();

            let p_word = if in_remaining {
                count as f64 / remaining_p as f64
            } else {
                0.0
            };
            let e_info = -sum;
            let goodness = -(p_word * (score + 1.0)
                + (1.0 - p_word) * (score + est_steps_left(remaining_entropy - e_info)));

            candidates.push(Candidate { word, goodness });

            if in_remaining {
                i += 1;
                if i >= stop {
                    break;
                }
            }
        }

        // Sort candidates by goodness in descending order
        candidates.sort_by(|a, b| b.goodness.partial_cmp(&a.goodness).unwrap());

        // We're not automatically setting last_guess_idx here anymore
        // Since we don't know which word the user will choose
        // It will be set in the next call based on the actual word chosen

        // Return the top 10 candidates (or fewer if there aren't 10)
        let count = candidates.len().min(10);
        candidates
            .iter()
            .take(count)
            .map(|c| GuessResult {
                word: c.word.to_string(),
                score: c.goodness,
            })
            .collect()
    }
}

#[derive(Debug, Copy, Clone)]
struct Candidate {
    word: &'static str,
    goodness: f64,
}
