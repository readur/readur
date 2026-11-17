//! Search-result snippets: short excerpts of a document's text around each
//! occurrence of a highlight term. All offsets are UTF-8 byte offsets (the
//! web UI converts them), and every slice lands on a char boundary.

use std::borrow::Cow;

use crate::models::{HighlightRange, SearchSnippet};

/// Most snippets taken for one term in one text.
const MAX_SNIPPETS_PER_TERM: usize = 3;
/// Most snippets returned for one document.
const MAX_SNIPPETS: usize = 5;

/// Snippets of about `snippet_length` bytes around case-insensitive matches
/// of `terms` in `texts`, searched in order.
pub(crate) fn build_snippets(
    texts: &[&str],
    terms: &[String],
    snippet_length: usize,
) -> Vec<SearchSnippet> {
    let mut snippets = Vec::new();
    for text in texts.iter().filter(|text| !text.is_empty()) {
        let folded = FoldedText::new(text);
        for term in terms {
            for (start, len) in folded.matches(term).take(MAX_SNIPPETS_PER_TERM) {
                snippets.push(snippet_around(text, start, len, snippet_length));
                if snippets.len() >= MAX_SNIPPETS {
                    return snippets;
                }
            }
        }
    }
    snippets
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
    fn matches(&self, term: &str) -> impl Iterator<Item = (usize, usize)> + '_ {
        let needle = if self.full_unicode {
            term.to_lowercase()
        } else {
            term.to_ascii_lowercase()
        };
        let len = needle.len();
        let haystack: &str = &self.folded;
        let mut from = 0;
        std::iter::from_fn(move || {
            if len == 0 {
                return None;
            }
            let start = from + haystack.get(from..)?.find(&needle)?;
            from = start + len;
            Some((start, len))
        })
    }
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
        let snippets = build_snippets(&[text], &terms(&["invoice", "receipt"]), 200);
        assert_eq!(highlighted(&snippets), ["Invoice", "invoice", "RECEIPT"]);
        assert_eq!(snippets[0].text, text);
        assert_eq!(snippets[0].start_offset, 0);
        assert_eq!(snippets[0].end_offset, text.len() as i32);
    }

    #[test]
    fn highlights_a_phrase_as_one_range() {
        let snippets = build_snippets(&["Amount: total due now"], &terms(&["total due"]), 200);
        assert_eq!(highlighted(&snippets), ["total due"]);
    }

    #[test]
    fn caps_snippets_per_term_and_per_document() {
        let text = "a ".repeat(10) + &"x ".repeat(10) + &"y ".repeat(10);
        assert_eq!(build_snippets(&[&text], &terms(&["x"]), 10).len(), 3);
        assert_eq!(build_snippets(&[&text], &terms(&["x", "y"]), 10).len(), 5);
        assert!(build_snippets(&[&text], &terms(&["zzz"]), 10).is_empty());
        assert!(build_snippets(&[&text], &[], 10).is_empty());
        assert!(build_snippets(&[""], &terms(&["x"]), 10).is_empty());
    }

    #[test]
    fn searches_every_text_in_order() {
        let snippets = build_snippets(
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
        let snippets = build_snippets(&[text], &terms(&["adipiscing"]), 20);
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
        let snippets = build_snippets(&[&text], &terms(&["needle"]), 4);
        assert_eq!(highlighted(&snippets), ["needle"]);
    }

    #[test]
    fn multibyte_text_never_splits_a_char() {
        // Each char here is 2-4 bytes, so byte arithmetic that ignores char
        // boundaries would panic on slicing.
        let text = "Ünïcödé тест 日本語のテキスト тест 🎉🎉 тест ÉLAN";
        for len in [1, 3, 7, 10, 20, 200] {
            let snippets =
                build_snippets(&[text], &terms(&["тест", "テキスト", "élan", "🎉"]), len);
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
        let snippets = build_snippets(&[text], &terms(&["invoice"]), 200);
        assert_eq!(highlighted(&snippets), ["INVOICE"]);
    }
}
