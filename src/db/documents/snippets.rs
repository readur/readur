//! Search-result snippets: short excerpts of a document's text around each
//! occurrence of a highlight term. All offsets are UTF-8 byte offsets (the
//! web UI converts them), and every slice lands on a char boundary.

use std::borrow::Cow;

use super::trigram::{words, FuzzyWordMatcher};
use crate::models::{FuzzyThreshold, HighlightRange, SearchMode, SearchSnippet};

/// Most snippets taken for one term in one text.
const MAX_SNIPPETS_PER_TERM: usize = 3;
/// Most snippets returned for one document.
const MAX_SNIPPETS: usize = 5;
/// Most bytes of a text read for fuzzy highlighting: scoring a word is far
/// costlier than a substring search, so a multi-megabyte text would
/// otherwise take seconds. Words past this point are not highlighted.
pub(crate) const MAX_FUZZY_SCAN_BYTES: usize = 512 * 1024;

/// How the highlighted stretches of a text are found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Highlight {
    /// Case-insensitive occurrences of each term as typed.
    Exact,
    /// Whole words whose pg_trgm word similarity to a word of a term
    /// reaches `cutoff`: the words a fuzzy search matched, which can be
    /// spelled differently from the query ("invoice" for "invoce").
    Fuzzy { cutoff: f64 },
}

impl Highlight {
    /// The highlighting that shows why a search in `mode` matched; `fuzzy`
    /// is the cutoff that search ran with.
    pub(crate) fn for_mode(mode: &SearchMode, fuzzy: FuzzyThreshold) -> Self {
        match mode {
            SearchMode::Fuzzy => Self::Fuzzy {
                cutoff: fuzzy.cutoff(),
            },
            _ => Self::Exact,
        }
    }
}

/// Snippets of about `snippet_length` bytes around the matches of `terms`
/// in `texts`, searched in order: each term's matches (at most
/// `MAX_SNIPPETS_PER_TERM` per text) in text order, terms in order.
pub(crate) fn build_snippets(
    texts: &[&str],
    terms: &[String],
    snippet_length: usize,
    highlight: Highlight,
) -> Vec<SearchSnippet> {
    let mut fuzzy = match highlight {
        Highlight::Exact => None,
        Highlight::Fuzzy { cutoff } => {
            let query_words: Vec<String> = terms
                .iter()
                .flat_map(|term| words(term).map(|(_, w)| w.to_string()))
                .collect();
            Some(FuzzyWordMatcher::new(&query_words, cutoff))
        }
    };
    let mut snippets = Vec::new();
    for text in texts.iter().filter(|text| !text.is_empty()) {
        let matches = match fuzzy.as_mut() {
            Some(matcher) => fuzzy_matches(text, matcher),
            None => exact_matches(text, terms),
        };
        for (start, len) in matches.into_iter().flatten() {
            snippets.push(snippet_around(text, start, len, snippet_length));
            if snippets.len() >= MAX_SNIPPETS {
                return snippets;
            }
        }
    }
    snippets
}

/// `(byte offset, byte length)` of each term's first matches in `text`.
fn exact_matches(text: &str, terms: &[String]) -> Vec<Vec<(usize, usize)>> {
    let folded = FoldedText::new(text);
    terms
        .iter()
        .map(|term| folded.matches(term).take(MAX_SNIPPETS_PER_TERM).collect())
        .collect()
}

/// The start of `text` fuzzy highlighting reads: at most
/// `MAX_FUZZY_SCAN_BYTES`, ending on a char boundary.
fn fuzzy_scan_window(text: &str) -> &str {
    &text[..text.floor_char_boundary(MAX_FUZZY_SCAN_BYTES)]
}

/// `(byte offset, byte length)` of the first words of `text` matching each
/// query word of `matcher`. Stops reading once every query word has its
/// fill, so a long text with early matches is not scanned to the end, and
/// in any case after `MAX_FUZZY_SCAN_BYTES`.
fn fuzzy_matches(text: &str, matcher: &mut FuzzyWordMatcher) -> Vec<Vec<(usize, usize)>> {
    let mut found = vec![Vec::new(); matcher.query_word_count()];
    let mut open = found.len();
    for (start, word) in words(fuzzy_scan_window(text)) {
        if open == 0 {
            break;
        }
        for (hits, matched) in found.iter_mut().zip(matcher.matches(word)) {
            if matched && hits.len() < MAX_SNIPPETS_PER_TERM {
                hits.push((start, word.len()));
                open -= usize::from(hits.len() == MAX_SNIPPETS_PER_TERM);
            }
        }
    }
    found
}

/// Lower-cased text whose byte offsets equal the original's. Full Unicode
/// lower-casing is used when it keeps every char the same width; otherwise
/// (e.g. `İ` widens) only ASCII is folded, which always keeps offsets.
struct FoldedText<'a> {
    folded: Cow<'a, str>,
    full_unicode: bool,
}

impl<'a> FoldedText<'a> {
    fn new(text: &'a str) -> Self {
        let lower = text.to_lowercase();
        let aligned = lower.len() == text.len()
            && text
                .char_indices()
                .zip(lower.char_indices())
                .all(|((a, _), (b, _))| a == b);
        if aligned {
            Self {
                folded: Cow::Owned(lower),
                full_unicode: true,
            }
        } else {
            Self {
                folded: Cow::Owned(text.to_ascii_lowercase()),
                full_unicode: false,
            }
        }
    }

    /// `(byte offset, byte length)` of each non-overlapping match of `term`.
    /// A term of several words matches them in order with any run of
    /// whitespace or punctuation between them, as the full-text matcher
    /// reads "total due" in "Total:\n due".
    fn matches(&self, term: &str) -> impl Iterator<Item = (usize, usize)> + '_ {
        let needles: Vec<String> = term
            .split_whitespace()
            .map(|word| {
                if self.full_unicode {
                    word.to_lowercase()
                } else {
                    word.to_ascii_lowercase()
                }
            })
            .collect();
        let haystack: &str = &self.folded;
        let mut from = 0;
        std::iter::from_fn(move || {
            let (first, rest) = needles.split_first()?;
            loop {
                let start = from + haystack.get(from..)?.find(first.as_str())?;
                if let Some(end) = phrase_end(haystack, start + first.len(), rest) {
                    from = end;
                    return Some((start, end - start));
                }
                // Retry from the next char: a later start may complete it.
                from = start + haystack[start..].chars().next().map_or(1, char::len_utf8);
            }
        })
    }
}

/// Where the words `rest` end when they follow `pos` in `haystack`, each
/// after at least one non-alphanumeric char; `None` if they do not.
fn phrase_end(haystack: &str, mut pos: usize, rest: &[String]) -> Option<usize> {
    for word in rest {
        let mut gap = false;
        loop {
            let tail = &haystack[pos..];
            if gap && tail.starts_with(word.as_str()) {
                break;
            }
            let c = tail.chars().next().filter(|c| !c.is_alphanumeric())?;
            pos += c.len_utf8();
            gap = true;
        }
        pos += word.len();
    }
    Some(pos)
}

/// The snippet containing the match at `start..start + len`, widened to
/// word boundaries.
fn snippet_around(text: &str, start: usize, len: usize, snippet_length: usize) -> SearchSnippet {
    let half = snippet_length / 2;
    let snippet_start = if start >= half {
        boundary_before(text, start - half)
    } else {
        0
    };
    let desired_end = snippet_start + snippet_length;
    let snippet_end = if desired_end < text.len() {
        boundary_after(text, desired_end)
    } else {
        text.len()
    }
    // A long run without boundaries can end the window before the match.
    .max(start + len);
    let highlight_start = start - snippet_start;
    SearchSnippet {
        text: text[snippet_start..snippet_end].to_string(),
        start_offset: snippet_start as i32,
        end_offset: snippet_end as i32,
        highlight_ranges: vec![HighlightRange {
            start: highlight_start as i32,
            end: (highlight_start + len) as i32,
        }],
    }
}

fn is_boundary(c: char) -> bool {
    c.is_whitespace() || c.is_ascii_punctuation()
}

/// Offset of the last boundary char at or before `pos`, or 0.
fn boundary_before(text: &str, pos: usize) -> usize {
    let pos = text.floor_char_boundary(pos);
    let end = text[pos..]
        .chars()
        .next()
        .map_or(pos, |c| pos + c.len_utf8());
    text[..end]
        .char_indices()
        .rev()
        .find(|&(_, c)| is_boundary(c))
        .map_or(0, |(i, _)| i)
}

/// Offset of the first boundary char at or after `pos`, or the text length.
fn boundary_after(text: &str, pos: usize) -> usize {
    let pos = text.ceil_char_boundary(pos);
    text[pos..]
        .char_indices()
        .find(|&(_, c)| is_boundary(c))
        .map_or(text.len(), |(i, _)| pos + i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms(list: &[&str]) -> Vec<String> {
        list.iter().map(|t| t.to_string()).collect()
    }

    fn exact(texts: &[&str], terms: &[String], len: usize) -> Vec<SearchSnippet> {
        build_snippets(texts, terms, len, Highlight::Exact)
    }

    fn fuzzy(texts: &[&str], terms: &[String], cutoff: f64) -> Vec<SearchSnippet> {
        build_snippets(texts, terms, 200, Highlight::Fuzzy { cutoff })
    }

    /// The highlighted part of each snippet.
    fn highlighted(snippets: &[SearchSnippet]) -> Vec<String> {
        snippets
            .iter()
            .map(|s| {
                let r = &s.highlight_ranges[0];
                s.text[r.start as usize..r.end as usize].to_string()
            })
            .collect()
    }

    #[test]
    fn highlights_each_term_case_insensitively() {
        let text = "Invoice 42 and a RECEIPT for the invoice.";
        let snippets = exact(&[text], &terms(&["invoice", "receipt"]), 200);
        assert_eq!(highlighted(&snippets), ["Invoice", "invoice", "RECEIPT"]);
        assert_eq!(snippets[0].text, text);
        assert_eq!(snippets[0].start_offset, 0);
        assert_eq!(snippets[0].end_offset, text.len() as i32);
    }

    #[test]
    fn highlights_a_phrase_as_one_range() {
        let snippets = exact(&["Amount: total due now"], &terms(&["total due"]), 200);
        assert_eq!(highlighted(&snippets), ["total due"]);
    }

    #[test]
    fn a_phrase_matches_across_any_whitespace_or_punctuation() {
        let text = "Amount TOTAL:\n\t due now; total-due; totaldue; total d";
        let snippets = exact(&[text], &terms(&["total  due"]), 200);
        assert_eq!(highlighted(&snippets), ["TOTAL:\n\t due", "total-due"]);
        // A later start can complete a phrase an earlier one could not.
        let snippets = exact(&["total total due"], &terms(&["total due"]), 200);
        assert_eq!(highlighted(&snippets), ["total due"]);
        assert_eq!(snippets[0].highlight_ranges[0].start, 6);
        assert!(exact(&["total"], &terms(&["total due"]), 200).is_empty());
        assert!(exact(&["anything"], &terms(&["  "]), 200).is_empty());
    }

    #[test]
    fn caps_snippets_per_term_and_per_document() {
        let text = "a ".repeat(10) + &"x ".repeat(10) + &"y ".repeat(10);
        assert_eq!(exact(&[&text], &terms(&["x"]), 10).len(), 3);
        assert_eq!(exact(&[&text], &terms(&["x", "y"]), 10).len(), 5);
        assert!(exact(&[&text], &terms(&["zzz"]), 10).is_empty());
        assert!(exact(&[&text], &[], 10).is_empty());
        assert!(exact(&[""], &terms(&["x"]), 10).is_empty());
    }

    #[test]
    fn searches_every_text_in_order() {
        let snippets = exact(
            &["first apple", "", "second apple"],
            &terms(&["apple"]),
            200,
        );
        assert_eq!(snippets.len(), 2);
        assert_eq!(snippets[1].text, "second apple");
    }

    #[test]
    fn windows_widen_to_word_boundaries() {
        let text = "lorem ipsum dolor sit amet consectetur adipiscing elit sed do";
        let snippets = exact(&[text], &terms(&["adipiscing"]), 20);
        let s = &snippets[0];
        assert!(s.text.contains("adipiscing"), "{}", s.text);
        assert_eq!(
            &text[s.start_offset as usize..s.end_offset as usize],
            s.text
        );
        assert_eq!(highlighted(&snippets), ["adipiscing"]);
    }

    #[test]
    fn a_match_longer_than_the_window_stays_inside_the_snippet() {
        let text = format!("{}needle{}", "a".repeat(50), "b".repeat(50));
        let snippets = exact(&[&text], &terms(&["needle"]), 4);
        assert_eq!(highlighted(&snippets), ["needle"]);
    }

    #[test]
    fn multibyte_text_never_splits_a_char() {
        // Each char here is 2-4 bytes, so byte arithmetic that ignores char
        // boundaries would panic on slicing.
        let text = "Ünïcödé тест 日本語のテキスト тест 🎉🎉 тест ÉLAN";
        for len in [1, 3, 7, 10, 20, 200] {
            let snippets = exact(&[text], &terms(&["тест", "テキスト", "élan", "🎉"]), len);
            for s in &snippets {
                assert_eq!(
                    &text[s.start_offset as usize..s.end_offset as usize],
                    s.text
                );
            }
            let hits = highlighted(&snippets);
            assert!(hits.contains(&"тест".to_string()), "{hits:?}");
            assert!(hits.contains(&"ÉLAN".to_string()), "{hits:?}");
        }
    }

    #[test]
    fn text_whose_lowercase_widens_still_matches_ascii() {
        // `İ` lower-cases to two chars, so full folding would shift offsets.
        let text = "İstanbul INVOICE İzmir";
        let snippets = exact(&[text], &terms(&["invoice"]), 200);
        assert_eq!(highlighted(&snippets), ["INVOICE"]);
    }

    #[test]
    fn fuzzy_highlights_the_words_a_misspelt_query_matched() {
        let text = "Please pay INVOICE 42 by Friday; invoices are due monthly.";
        let snippets = fuzzy(&[text], &terms(&["invoce"]), 0.5);
        assert_eq!(highlighted(&snippets), ["INVOICE", "invoices"]);
        // The typed spelling occurs nowhere, so exact highlighting finds nothing.
        assert!(exact(&[text], &terms(&["invoce"]), 200).is_empty());
    }

    #[test]
    fn fuzzy_honours_the_cutoff() {
        let text = "invoice";
        // word_similarity('invoce', 'invoice') is 0.5714286 in Postgres.
        assert_eq!(fuzzy(&[text], &terms(&["invoce"]), 0.57).len(), 1);
        assert!(fuzzy(&[text], &terms(&["invoce"]), 0.58).is_empty());
        // Exact spellings always reach the strictest cutoff.
        assert_eq!(
            highlighted(&fuzzy(&[text], &terms(&["Invoice"]), 1.0)),
            ["invoice"]
        );
    }

    #[test]
    fn fuzzy_matches_each_query_word_and_splits_terms_like_pg_trgm() {
        let text = "The total of invoice-42 is due.";
        let snippets = fuzzy(&[text], &terms(&["invoce-42", "totl"]), 0.5);
        assert_eq!(highlighted(&snippets), ["invoice", "42", "total"]);
        assert!(fuzzy(&[text], &terms(&["--"]), 0.5).is_empty());
        assert!(fuzzy(&[text], &[], 0.5).is_empty());
    }

    #[test]
    fn fuzzy_caps_snippets_like_exact_mode() {
        let text = "invoice ".repeat(10) + &"total ".repeat(10);
        assert_eq!(fuzzy(&[&text], &terms(&["invoce"]), 0.5).len(), 3);
        assert_eq!(fuzzy(&[&text], &terms(&["invoce", "totl"]), 0.5).len(), 5);
    }

    #[test]
    fn fuzzy_offsets_land_on_multibyte_words() {
        let text = "Grüße aus München, ÜNÏCODE тесты";
        let snippets = fuzzy(&[text], &terms(&["munchen", "тест", "Ünïcode"]), 0.6);
        for s in &snippets {
            assert_eq!(
                &text[s.start_offset as usize..s.end_offset as usize],
                s.text
            );
        }
        assert_eq!(highlighted(&snippets), ["тесты", "ÜNÏCODE"]);
    }

    #[test]
    fn fuzzy_scanning_stops_at_the_byte_cap() {
        // Distinct words, so the matcher scores (and caches) every word it
        // reads; well over the cap in total.
        let text: String = (0..1_000_000).map(|i| format!("w{i} ")).collect();
        assert!(text.len() > 4 * MAX_FUZZY_SCAN_BYTES);
        let window = fuzzy_scan_window(&text);
        assert!(window.len() <= MAX_FUZZY_SCAN_BYTES);
        assert!(window.len() > MAX_FUZZY_SCAN_BYTES - 16);

        let mut matcher = FuzzyWordMatcher::new(&terms(&["nomatch"]), 0.9);
        let found = fuzzy_matches(&text, &mut matcher);
        assert!(found.iter().all(Vec::is_empty));
        // Every word read was scored once; none past the cap was read.
        let scored = matcher.scored_words();
        assert!(scored <= MAX_FUZZY_SCAN_BYTES / 3, "{scored} words scored");
        assert!(scored > MAX_FUZZY_SCAN_BYTES / 16, "{scored} words scored");

        // A match before the cap is found, one after it is not.
        let early = format!("invoice {text}");
        assert_eq!(fuzzy(&[&early], &terms(&["invoce"]), 0.5).len(), 1);
        let late = format!("{text} invoice");
        assert!(fuzzy(&[&late], &terms(&["invoce"]), 0.5).is_empty());
    }

    #[test]
    fn multi_megabyte_texts_yield_snippets_quickly() {
        let filler = "lorem ipsum dolor sit amet ".repeat(200_000);
        assert!(filler.len() > 5_000_000);
        let text = format!("{filler} invoice");
        let started = std::time::Instant::now();
        // Neither a fuzzy query word matching nothing nor an exact term
        // near the end may take long.
        assert!(fuzzy(&[&text, &text], &terms(&["zzzz"]), 0.9).is_empty());
        assert_eq!(exact(&[&text], &terms(&["invoice"]), 50).len(), 1);
        let elapsed = started.elapsed();
        assert!(elapsed.as_secs() < 5, "took {elapsed:?}");
    }

    #[test]
    fn highlight_mode_follows_the_search_mode() {
        let fuzzy = FuzzyThreshold::from_setting(Some(0.57));
        assert_eq!(
            Highlight::for_mode(&SearchMode::Fuzzy, fuzzy),
            Highlight::Fuzzy { cutoff: 0.57 }
        );
        for mode in [SearchMode::Simple, SearchMode::Phrase, SearchMode::Boolean] {
            assert_eq!(Highlight::for_mode(&mode, fuzzy), Highlight::Exact);
        }
    }
}
