//! Whole text nodes from quick-xml.
//!
//! Since quick-xml 0.38 a text node is no longer one `Event::Text`: every
//! entity or character reference inside it arrives as its own
//! `Event::GeneralRef`, so `Tom&amp;Jerry.pdf` reads as `Text("Tom")`,
//! `GeneralRef("amp")`, `Text("Jerry.pdf")`. Code written for one text event
//! per node then keeps only the last fragment (a WebDAV href becomes
//! `Jerry.pdf`) or splits values apart, and per-fragment trimming eats the
//! spaces around each reference.
//!
//! [`MergedTextReader`] restores the old contract: each run of text and
//! references comes back as a single `Event::Text` holding the resolved text,
//! trimmed only at the ends of the run if the wrapped reader was configured to
//! trim.

use quick_xml::escape::{resolve_predefined_entity, EscapeError};
use quick_xml::events::{BytesRef, BytesText, Event};
use quick_xml::{Reader, Result};
use std::io::BufRead;

/// A [`Reader`] that yields each text node as one already-resolved `Event::Text`.
///
/// Read the text with `BytesText::xml10_content()`; it contains no references.
pub struct MergedTextReader<R> {
    inner: Reader<R>,
    pending: Option<Event<'static>>,
    trim_start: bool,
    trim_end: bool,
}

impl<R: BufRead> MergedTextReader<R> {
    /// Wraps `inner`, taking over its `trim_text` setting: trimming is applied
    /// to the merged text node instead of to each fragment.
    pub fn new(mut inner: Reader<R>) -> Self {
        let (trim_start, trim_end) = (inner.config().trim_text_start, inner.config().trim_text_end);
        inner.config_mut().trim_text(false);
        Self { inner, pending: None, trim_start, trim_end }
    }

    /// Like [`Reader::read_event_into`], but text nodes come back whole.
    pub fn read_event_into(&mut self, buf: &mut Vec<u8>) -> Result<Event<'static>> {
        loop {
            let mut event = match self.pending.take() {
                Some(event) => event,
                None => self.next_owned(buf)?,
            };
            if !matches!(event, Event::Text(_) | Event::GeneralRef(_)) {
                return Ok(event);
            }
            // Literal fragments and resolved references, kept apart so trimming can
            // treat references as opaque content, as trimming the raw text did.
            let mut pieces: Vec<(bool, String)> = Vec::new();
            loop {
                match event {
                    Event::Text(t) => pieces.push((false, t.xml10_content().into_owned())),
                    Event::GeneralRef(r) => {
                        let mut resolved = String::new();
                        resolve_reference(&r, &mut resolved)?;
                        pieces.push((true, resolved));
                    }
                    other => {
                        self.pending = Some(other);
                        break;
                    }
                }
                event = self.next_owned(buf)?;
            }
            if self.trim_start {
                trim_edge(&mut pieces, |s| s.trim_start_matches(is_xml_space).len(), true);
            }
            if self.trim_end {
                trim_edge(&mut pieces, |s| s.trim_end_matches(is_xml_space).len(), false);
            }
            // A trimming reader never reported whitespace-only text; keep that.
            let trimming = self.trim_start || self.trim_end;
            if !(trimming && pieces.is_empty()) {
                let text: String = pieces.into_iter().map(|(_, piece)| piece).collect();
                // Already resolved: read it back with xml10_content(), which only
                // normalizes line endings (a literal `&#13;` would become `\n`).
                return Ok(Event::Text(BytesText::from_escaped(text)));
            }
        }
    }

    fn next_owned(&mut self, buf: &mut Vec<u8>) -> Result<Event<'static>> {
        buf.clear();
        Ok(self.inner.read_event_into(buf)?.into_owned())
    }
}

/// XML whitespace (S in the XML spec): what quick-xml's `trim_text` strips.
/// Unlike `char::is_whitespace`, this leaves U+00A0 and other Unicode spaces.
fn is_xml_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r' | '\n')
}

/// Strips XML whitespace from the literal pieces at one edge of a text run,
/// stopping at the first resolved reference or non-blank literal. `kept_len`
/// returns how much of a piece survives; it is cut from the end of the piece
/// when trimming the start, and kept from the start when trimming the end.
fn trim_edge(pieces: &mut Vec<(bool, String)>, kept_len: impl Fn(&str) -> usize, start: bool) {
    loop {
        let idx = if start { 0 } else { match pieces.len() { 0 => return, n => n - 1 } };
        let Some((is_ref, piece)) = pieces.get_mut(idx) else { return };
        if *is_ref {
            return;
        }
        let keep = kept_len(piece);
        if keep > 0 {
            if start {
                piece.drain(..piece.len() - keep);
            } else {
                piece.truncate(keep);
            }
            return;
        }
        pieces.remove(idx);
    }
}

/// Appends what `reference` stands for: a character reference or one of the
/// five predefined XML entities. Anything else is an error, as it was when
/// text was unescaped in one piece.
fn resolve_reference(reference: &BytesRef<'_>, out: &mut String) -> Result<()> {
    if let Some(ch) = reference.resolve_char_ref()? {
        out.push(ch);
        return Ok(());
    }
    let name = reference.xml10_content();
    match resolve_predefined_entity(&name) {
        Some(resolved) => {
            out.push_str(resolved);
            Ok(())
        }
        None => Err(EscapeError::UnrecognizedEntity(0..name.len(), name.into_owned()).into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every event, with text nodes as their resolved strings.
    fn events(xml: &str, trim: bool) -> Vec<String> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(trim);
        let mut reader = MergedTextReader::new(reader);
        let mut buf = Vec::new();
        let mut out = Vec::new();
        loop {
            match reader.read_event_into(&mut buf).unwrap() {
                Event::Eof => return out,
                Event::Text(t) => out.push(format!("text:{}", t.xml10_content())),
                Event::Start(e) => out.push(format!("start:{}", e.name().as_ref())),
                Event::End(e) => out.push(format!("end:{}", e.name().as_ref())),
                other => out.push(format!("{other:?}")),
            }
        }
    }

    #[test]
    fn entity_inside_a_node_stays_in_one_text_event() {
        assert_eq!(
            events("<href>/dav/Tom&amp;Jerry.pdf</href>", false),
            ["start:href", "text:/dav/Tom&Jerry.pdf", "end:href"]
        );
    }

    #[test]
    fn resolves_predefined_and_character_references() {
        assert_eq!(
            events("<t>&lt;a&gt; &quot;b&apos; &#233;&#x4E2D;</t>", false),
            ["start:t", "text:<a> \"b' é中", "end:t"]
        );
    }

    #[test]
    fn trims_the_merged_node_not_each_fragment() {
        assert_eq!(
            events("<t>  Tom &amp; Jerry  </t>", true),
            ["start:t", "text:Tom & Jerry", "end:t"]
        );
    }

    #[test]
    fn trimming_reader_still_drops_whitespace_only_text() {
        assert_eq!(
            events("<a>\n  <b>x</b>\n</a>", true),
            ["start:a", "start:b", "text:x", "end:b", "end:a"]
        );
    }

    #[test]
    fn non_trimming_reader_keeps_whitespace() {
        assert_eq!(events("<t> a </t>", false), ["start:t", "text: a ", "end:t"]);
    }

    #[test]
    fn a_node_that_is_only_a_reference() {
        assert_eq!(events("<t>&amp;</t>", false), ["start:t", "text:&", "end:t"]);
    }

    #[test]
    fn trimming_keeps_unicode_spaces_like_quick_xml_037() {
        // trim_text strips XML whitespace only; U+00A0 is content (e.g. a DOCX run).
        assert_eq!(events("<t>\u{a0}</t>", true), ["start:t", "text:\u{a0}", "end:t"]);
        assert_eq!(events("<t> \u{a0}Foo </t>", true), ["start:t", "text:\u{a0}Foo", "end:t"]);
    }

    #[test]
    fn trimming_never_strips_resolved_references() {
        // 0.37 trimmed the raw text, where `&#32;` is not whitespace.
        assert_eq!(events("<t>&#32;</t>", true), ["start:t", "text: ", "end:t"]);
        assert_eq!(events("<t>  &#160;x&#10;  </t>", true), ["start:t", "text:\u{a0}x\n", "end:t"]);
    }

    #[test]
    fn honours_start_and_end_trimming_separately() {
        let mut reader = Reader::from_str("<t>  a  </t>");
        reader.config_mut().trim_text_start = true;
        let mut reader = MergedTextReader::new(reader);
        let mut buf = Vec::new();
        reader.read_event_into(&mut buf).unwrap();
        match reader.read_event_into(&mut buf).unwrap() {
            Event::Text(t) => assert_eq!(t.xml10_content(), "a  "),
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[test]
    fn unknown_entity_is_an_error() {
        let mut reader = MergedTextReader::new(Reader::from_str("<t>a&bogus;b</t>"));
        let mut buf = Vec::new();
        reader.read_event_into(&mut buf).unwrap();
        assert!(reader.read_event_into(&mut buf).is_err());
    }
}
