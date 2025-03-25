use crate::{Correctness, Guess, PackedCorrectness, DICTIONARY, MAX_MASK_ENUM};
use once_cell::sync::OnceCell;
use once_cell::unsync::OnceCell as UnSyncOnceCell;
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
    pub fn guess(&mut self, history: &[Guess]) -> String {
        let score = history.len() as f64;

        if let Some(last) = history.last() {
            let reference = PackedCorrectness::from(last.mask);
            COMPUTES.with(|c| {
                let row = &c.get().unwrap()[self.last_guess_idx.unwrap()];
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
            return FIRST_GUESS.to_string();
        } else if self.remaining.len() == 1 {
            let w = self.remaining.first().unwrap();
            self.last_guess_idx = Some(w.2);
            return w.0.to_string();
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

        let mut best: Option<Candidate> = None;
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
            if let Some(c) = best {
                if goodness > c.goodness {
                    best = Some(Candidate {
                        word,
                        goodness,
                        idx: word_idx,
                    });
                }
            } else {
                best = Some(Candidate {
                    word,
                    goodness,
                    idx: word_idx,
                });
            }

            if in_remaining {
                i += 1;
                if i >= stop {
                    break;
                }
            }
        }
        let best = best.unwrap();
        assert_ne!(best.goodness, 0.0);
        self.last_guess_idx = Some(best.idx);
        best.word.to_string()
    }
}

#[derive(Debug, Copy, Clone)]
struct Candidate {
    word: &'static str,
    goodness: f64,
    idx: usize,
}
