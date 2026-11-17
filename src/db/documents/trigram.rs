//! A port of pg_trgm's trigram matching, so snippets can highlight the
//! words a fuzzy search actually matched ("invoice" for the query
//! "invoce") with the same scores Postgres used to match the document.
//!
//! Like pg_trgm: text is split into words of alphanumeric chars, each word
//! is lower-cased and padded with two leading blanks and one trailing blank,
//! and a word contributes every run of three consecutive chars. pg_trgm
//! stores a multi-byte trigram as a 3-byte hash; this port compares the
//! chars themselves, which differs only on (vanishingly rare) hash
//! collisions.

use std::collections::{HashMap, HashSet};

type Trigram = [char; 3];

/// Whether pg_trgm counts `c` as part of a word.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

/// `(byte offset, word)` of each word of `text`, in order.
pub(crate) fn words(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut rest = text.char_indices().peekable();
    std::iter::from_fn(move || {
        let (start, _) = rest.find(|&(_, c)| is_word_char(c))?;
        let mut end = text.len();
        while let Some(&(i, c)) = rest.peek() {
            if !is_word_char(c) {
                end = i;
                break;
            }
            rest.next();
        }
        Some((start, &text[start..end]))
    })
}

/// The trigrams of `text` in order, duplicates kept (pg_trgm's
/// `generate_trgm_only`).
fn trigrams(text: &str) -> Vec<Trigram> {
    let mut out = Vec::new();
    for (_, word) in words(text) {
        let padded: Vec<char> = "  "
            .chars()
            .chain(word.chars().flat_map(char::to_lowercase))
            .chain(std::iter::once(' '))
            .collect();
        out.extend(padded.windows(3).map(|w| [w[0], w[1], w[2]]));
    }
    out
}

/// pg_trgm's `CALCSML`: `count` shared trigrams out of `len1` and `len2`.
fn calc_similarity(count: usize, len1: usize, len2: usize) -> f32 {
    let union = len1 + len2 - count;
    if union == 0 {
        0.0
    } else {
        count as f32 / union as f32
    }
}

/// pg_trgm's `similarity(a, b)`: shared trigrams over all distinct trigrams
/// of both strings.
#[cfg(test)]
pub(crate) fn similarity(a: &str, b: &str) -> f32 {
    let a: HashSet<Trigram> = trigrams(a).into_iter().collect();
    let b: HashSet<Trigram> = trigrams(b).into_iter().collect();
    calc_similarity(a.intersection(&b).count(), a.len(), b.len())
}

/// pg_trgm's `word_similarity(query, text)`: the greatest similarity between
/// the trigram set of `query` and a continuous extent of the ordered
/// trigrams of `text`. This mirrors `iterate_word_similarity` step for step
/// (including its greedy choice of extent) so it returns the exact score
/// Postgres computes, not merely a close one. (`FuzzyWordMatcher` calls
/// `word_similarity_to_set` directly, reusing each query word's trigrams.)
#[cfg(test)]
pub(crate) fn word_similarity(query: &str, text: &str) -> f32 {
    let query: HashSet<Trigram> = trigrams(query).into_iter().collect();
    word_similarity_to_set(&query, &trigrams(text))
}

fn word_similarity_to_set(query: &HashSet<Trigram>, text: &[Trigram]) -> f32 {
    // Number each distinct trigram of `text`; `found` says whether it is
    // also one of the query's.
    let mut ids: HashMap<Trigram, usize> = HashMap::new();
    let mut found = Vec::new();
    let index: Vec<usize> = text
        .iter()
        .map(|t| {
            *ids.entry(*t).or_insert_with(|| {
                found.push(query.contains(t));
                found.len() - 1
            })
        })
        .collect();
    iterate_word_similarity(&index, &found, query.len())
}

/// `iterate_word_similarity` of pg_trgm (plain, not strict, word similarity).
fn iterate_word_similarity(index: &[usize], found: &[bool], ulen1: usize) -> f32 {
    let mut lastpos: Vec<Option<usize>> = vec![None; found.len()];
    let (mut ulen2, mut count) = (0usize, 0usize);
    let mut lower: Option<usize> = None;
    let mut best = 0.0f32;

    for (i, &id) in index.iter().enumerate() {
        if lower.is_some() || found[id] {
            if lastpos[id].is_none() {
                ulen2 += 1;
                count += usize::from(found[id]);
            }
            lastpos[id] = Some(i);
        }
        if !found[id] {
            continue;
        }
        let prev_lower = match lower {
            Some(l) => l,
            None => {
                ulen2 = 1;
                i
            }
        };
        let mut low = prev_lower;
        let mut current = calc_similarity(count, ulen1, ulen2);

        // Try moving the lower bound up for a greater similarity.
        let (mut tmp_count, mut tmp_ulen2) = (count, ulen2);
        for (tmp_lower, &tmp_id) in index.iter().enumerate().take(i + 1).skip(prev_lower) {
            let candidate = calc_similarity(tmp_count, ulen1, tmp_ulen2);
            if candidate > current {
                current = candidate;
                ulen2 = tmp_ulen2;
                low = tmp_lower;
                count = tmp_count;
            }
            if lastpos[tmp_id] == Some(tmp_lower) {
                tmp_ulen2 -= 1;
                tmp_count -= usize::from(found[tmp_id]);
            }
        }
        best = best.max(current);

        for (tmp_lower, &tmp_id) in index.iter().enumerate().take(low).skip(prev_lower) {
            if lastpos[tmp_id] == Some(tmp_lower) {
                lastpos[tmp_id] = None;
            }
        }
        lower = Some(low);
    }
    best
}

/// Finds the words of a text that a fuzzy query matched: those whose word
/// similarity to a query word reaches the search's cutoff.
pub(crate) struct FuzzyWordMatcher {
    /// The trigram set of each query word.
    query_words: Vec<HashSet<Trigram>>,
    cutoff: f64,
    /// Scores of text words already seen, by lower-cased word.
    cache: HashMap<String, Vec<f32>>,
}

impl FuzzyWordMatcher {
    /// `query_words` are matched one by one; `cutoff` is the value of
    /// `pg_trgm.word_similarity_threshold` the search ran with. Postgres
    /// compares its single-precision score against that double, and so
    /// does this.
    pub(crate) fn new(query_words: &[String], cutoff: f64) -> Self {
        Self {
            query_words: query_words
                .iter()
                .map(|w| trigrams(w).into_iter().collect())
                .collect(),
            cutoff,
            cache: HashMap::new(),
        }
    }

    /// How many distinct text words have been scored.
    #[cfg(test)]
    pub(crate) fn scored_words(&self) -> usize {
        self.cache.len()
    }

    pub(crate) fn query_word_count(&self) -> usize {
        self.query_words.len()
    }

    /// For each query word, whether `word` (one word of the text) matches it.
    pub(crate) fn matches(&mut self, word: &str) -> impl Iterator<Item = bool> + '_ {
        let key = word.to_lowercase();
        if !self.cache.contains_key(&key) {
            let text = trigrams(&key);
            let scores = self
                .query_words
                .iter()
                .map(|q| word_similarity_to_set(q, &text))
                .collect();
            self.cache.insert(key.clone(), scores);
        }
        let cutoff = self.cutoff;
        self.cache[&key]
            .iter()
            .map(move |&score| f64::from(score) >= cutoff)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `(a, b, similarity(a, b), word_similarity(a, b))` as computed by
    /// Postgres 16's pg_trgm (`SELECT similarity(a, b), word_similarity(a, b)`).
    const PG_SCORES: &[(&str, &str, f32, f32)] = &include!("trigram_pg_scores.in");

    #[test]
    fn matches_postgres_scores() {
        for &(a, b, sim, word_sim) in PG_SCORES {
            assert_eq!(similarity(a, b), sim, "similarity({a:?}, {b:?})");
            assert_eq!(
                word_similarity(a, b),
                word_sim,
                "word_similarity({a:?}, {b:?})"
            );
        }
    }

    #[test]
    fn words_split_on_non_alphanumerics_with_byte_offsets() {
        let found: Vec<_> = words("  Invoice-42, für Ünïcode!").collect();
        assert_eq!(
            found,
            [(2, "Invoice"), (10, "42"), (14, "für"), (19, "Ünïcode")]
        );
        assert_eq!(words("").count(), 0);
        assert_eq!(words(" -- ").count(), 0);
    }

    #[test]
    fn trigrams_pad_each_lower_cased_word() {
        let t: Vec<String> = trigrams("Ab c")
            .iter()
            .map(|t| t.iter().collect())
            .collect();
        assert_eq!(t, ["  a", " ab", "ab ", "  c", " c "]);
    }

    #[test]
    fn matcher_applies_the_cutoff_per_query_word() {
        let mut m = FuzzyWordMatcher::new(&["invoce".into(), "totl".into()], 0.5);
        assert_eq!(m.query_word_count(), 2);
        assert_eq!(m.matches("INVOICE").collect::<Vec<_>>(), [true, false]);
        assert_eq!(m.matches("total").collect::<Vec<_>>(), [false, true]);
        assert_eq!(m.matches("receipt").collect::<Vec<_>>(), [false, false]);
        // A score exactly at the cutoff matches, as with `<%`.
        let score = word_similarity("invoce", "invoice");
        let mut at = FuzzyWordMatcher::new(&["invoce".into()], f64::from(score));
        assert!(at.matches("invoice").all(|m| m));
        let mut above = FuzzyWordMatcher::new(&["invoce".into()], f64::from(score) + 1e-6);
        assert!(!above.matches("invoice").any(|m| m));
    }
}
