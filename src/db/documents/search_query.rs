//! Parser for boolean-mode search input.
//!
//! Users and AI clients write boolean queries in several dialects, and raw
//! `to_tsquery` input raises a syntax error on most of them. This module
//! parses the input into an AST with a forgiving recursive-descent parser and
//! renders it back as `to_tsquery` syntax that cannot fail to parse: every
//! lexeme is quoted and escaped, every compound is parenthesised, and
//! malformed input degrades (dropped operators, implicitly closed groups)
//! instead of erroring.
//!
//! Grammar, loosest binding first:
//!
//! ```text
//! or       := and ( ("|" | OR) and )*
//! and      := sequence ( ["&" | AND] sequence )*      adjacency means AND
//! sequence := unary ( ("<->" | "<N>") unary )*        followed-by (distance N)
//! unary    := ("!" | NOT | "-")* primary              "-" only when attached
//! primary  := "(" or ")" | "\"phrase\"" | word[":*" | "*"]
//! ```
//!
//! `AND`, `OR` and `NOT` are recognised in any case: as search terms they are
//! English stop words that could never match anyway.

use std::iter::Peekable;
use std::str::Chars;

use crate::db::sanitize::strip_nul;

/// Deepest parenthesis nesting honoured; deeper groups are flattened into
/// their parent so neither this parser nor Postgres recurses without bound.
const MAX_GROUP_DEPTH: usize = 32;
/// Largest distance Postgres accepts in a `<N>` operator.
const MAX_DISTANCE: u16 = 16384;
/// Characters that end a bare word.
const WORD_TERMINATORS: &str = "()&|!\"<";

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Phrase(String),
    Open,
    Close,
    And,
    Or,
    Not,
    FollowedBy(u16),
}

/// A parsed boolean query.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Node {
    /// One search word; `prefix` matches every word starting with it.
    Term {
        text: String,
        prefix: bool,
    },
    /// A quoted phrase of two or more words, matched in order.
    Phrase(Vec<String>),
    Not(Box<Node>),
    And(Vec<Node>),
    Or(Vec<Node>),
    /// `first` followed by each operand at the given distance.
    Sequence {
        first: Box<Node>,
        rest: Vec<(u16, Node)>,
    },
}

/// Boolean search input, parsed. Parsing never fails: input without a single
/// searchable word yields an empty query, which matches nothing.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BooleanQuery {
    root: Option<Node>,
}

/// Renders boolean search input as `to_tsquery` input that Postgres always
/// accepts; see the module docs for the syntax. This is what boolean-mode
/// search binds as its query.
pub fn boolean_to_tsquery(input: &str) -> String {
    BooleanQuery::parse(input).to_tsquery()
}

impl BooleanQuery {
    /// Parses `input`; NUL characters (which Postgres text cannot hold) are
    /// removed first.
    pub(crate) fn parse(input: &str) -> Self {
        let mut parser = Parser {
            tokens: tokenize(&strip_nul(input)),
            pos: 0,
        };
        let root = parser.parse_or();
        debug_assert_eq!(parser.pos, parser.tokens.len(), "unconsumed tokens");
        Self { root }
    }

    #[cfg(test)]
    pub(crate) fn root(&self) -> Option<&Node> {
        self.root.as_ref()
    }

    /// The query as `to_tsquery` input. An empty query renders as the empty
    /// string, which `to_tsquery` turns into an empty tsquery matching
    /// nothing (with a NOTICE, not an error).
    pub(crate) fn to_tsquery(&self) -> String {
        self.root.as_ref().map(render).unwrap_or_default()
    }

    /// The words and phrases a matching document contains: every term that
    /// is not negated, without operators, quotes or prefix markers. Used to
    /// highlight snippets.
    pub(crate) fn positive_terms(&self) -> Vec<String> {
        let mut terms = Vec::new();
        if let Some(root) = &self.root {
            collect_positive(root, false, &mut terms);
        }
        terms
    }
}

// ---------------------------------------------------------------- lexing

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        let token = match c {
            '(' => Some(Token::Open),
            ')' => Some(Token::Close),
            '&' => Some(Token::And),
            '|' => Some(Token::Or),
            '!' => Some(Token::Not),
            '"' => Some(Token::Phrase(
                chars.by_ref().take_while(|&c| c != '"').collect(),
            )),
            '<' => followed_by(&mut chars),
            // A minus negates only when attached: `-draft`, `-"a b"`, `-(a)`.
            '-' => chars
                .peek()
                .filter(|next| !next.is_whitespace())
                .map(|_| Token::Not),
            c if c.is_whitespace() => None,
            c => Some(word(c, &mut chars)),
        };
        tokens.extend(token);
    }
    balance_groups(tokens)
}

/// Reads the rest of `<->` or `<N>` after a `<`. Anything else leaves the
/// input untouched and the `<` is ignored.
fn followed_by(chars: &mut Peekable<Chars>) -> Option<Token> {
    let mut probe = chars.clone();
    let distance = if probe.next_if_eq(&'-').is_some() {
        1
    } else {
        let digits: String = std::iter::from_fn(|| probe.next_if(char::is_ascii_digit)).collect();
        if digits.is_empty() {
            return None;
        }
        digits
            .parse::<u16>()
            .map_or(MAX_DISTANCE, |d| d.min(MAX_DISTANCE))
    };
    probe.next_if_eq(&'>')?;
    *chars = probe;
    Some(Token::FollowedBy(distance))
}

/// A bare word, or the operator keyword it spells.
fn word(first: char, chars: &mut Peekable<Chars>) -> Token {
    let mut word = String::from(first);
    while let Some(c) = chars.next_if(|&c| !c.is_whitespace() && !WORD_TERMINATORS.contains(c)) {
        word.push(c);
    }
    match word.to_ascii_uppercase().as_str() {
        "AND" => Token::And,
        "OR" => Token::Or,
        "NOT" => Token::Not,
        _ => Token::Word(word),
    }
}

/// Drops `)` without a matching `(`, and groups nested deeper than
/// `MAX_GROUP_DEPTH` (both parentheses). An unclosed `(` is left for the
/// parser, which closes it at the end of the input.
fn balance_groups(tokens: Vec<Token>) -> Vec<Token> {
    // One entry per open group: whether its parentheses are kept.
    let mut open: Vec<bool> = Vec::new();
    let mut depth = 0;
    tokens
        .into_iter()
        .filter(|token| match token {
            Token::Open => {
                let keep = depth < MAX_GROUP_DEPTH;
                depth += usize::from(keep);
                open.push(keep);
                keep
            }
            Token::Close => {
                let keep = open.pop().unwrap_or(false);
                depth -= usize::from(keep);
                keep
            }
            _ => true,
        })
        .collect()
}

// --------------------------------------------------------------- parsing

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn eat(&mut self, token: &Token) -> bool {
        let matches = self.peek() == Some(token);
        self.pos += usize::from(matches);
        matches
    }

    fn parse_or(&mut self) -> Option<Node> {
        let mut alternatives = Vec::new();
        loop {
            alternatives.extend(self.parse_and());
            if !self.eat(&Token::Or) {
                return combine(alternatives, Node::Or);
            }
        }
    }

    /// Operands joined by `&`, `AND` or plain adjacency. Stray `&` are
    /// skipped; the group ends at `|`, `)` or the end of the input.
    fn parse_and(&mut self) -> Option<Node> {
        let mut operands = Vec::new();
        loop {
            match self.peek() {
                None | Some(Token::Or | Token::Close) => return combine(operands, Node::And),
                Some(Token::And) => self.pos += 1,
                Some(_) => operands.extend(self.parse_sequence()),
            }
        }
    }

    /// Operands joined by followed-by operators. An operator missing an
    /// operand on either side is dropped.
    fn parse_sequence(&mut self) -> Option<Node> {
        let mut first = self.parse_unary();
        let mut rest = Vec::new();
        while let Some(&Token::FollowedBy(distance)) = self.peek() {
            self.pos += 1;
            if let Some(next) = self.parse_unary() {
                if first.is_none() {
                    first = Some(next);
                } else {
                    rest.push((distance, next));
                }
            }
        }
        let first = first?;
        Some(if rest.is_empty() {
            first
        } else {
            Node::Sequence {
                first: Box::new(first),
                rest,
            }
        })
    }

    /// Any number of negations (an even count cancels out) and an operand.
    fn parse_unary(&mut self) -> Option<Node> {
        let mut negated = false;
        while self.eat(&Token::Not) {
            negated = !negated;
        }
        let node = self.parse_primary()?;
        Some(if negated { negate(node) } else { node })
    }

    fn parse_primary(&mut self) -> Option<Node> {
        let node = match self.peek()?.clone() {
            Token::Open => {
                self.pos += 1;
                let inner = self.parse_or();
                self.eat(&Token::Close);
                return inner;
            }
            Token::Word(word) => term(&word),
            Token::Phrase(phrase) => phrase_node(&phrase),
            _ => return None,
        };
        self.pos += 1;
        node
    }
}

/// `nodes` joined by `join`; one node stands alone, none is nothing.
fn combine(mut nodes: Vec<Node>, join: fn(Vec<Node>) -> Node) -> Option<Node> {
    match nodes.len() {
        0 => None,
        1 => nodes.pop(),
        _ => Some(join(nodes)),
    }
}

fn negate(node: Node) -> Node {
    match node {
        Node::Not(inner) => *inner,
        node => Node::Not(Box::new(node)),
    }
}

/// Whether `text` holds anything Postgres could index as a word.
fn has_lexeme(text: &str) -> bool {
    text.chars().any(char::is_alphanumeric)
}

/// A word, with a trailing `:*` or `*` making it a prefix match.
fn term(word: &str) -> Option<Node> {
    let stripped = word.trim_end_matches('*');
    let prefix = stripped.len() != word.len();
    let text = if prefix {
        stripped.strip_suffix(':').unwrap_or(stripped)
    } else {
        word
    };
    has_lexeme(text).then(|| Node::Term {
        text: text.to_string(),
        prefix,
    })
}

fn phrase_node(phrase: &str) -> Option<Node> {
    let mut words: Vec<String> = phrase
        .split_whitespace()
        .filter(|w| has_lexeme(w))
        .map(String::from)
        .collect();
    match words.len() {
        0 => None,
        1 => words.pop().map(|text| Node::Term {
            text,
            prefix: false,
        }),
        _ => Some(Node::Phrase(words)),
    }
}

// ------------------------------------------------------------- rendering

fn render(node: &Node) -> String {
    match node {
        Node::Term { text, prefix } => {
            let marker = if *prefix { ":*" } else { "" };
            format!("{}{marker}", quote_lexeme(text))
        }
        Node::Phrase(words) => group(words.iter().map(|w| quote_lexeme(w)), " <-> "),
        Node::Not(inner) => format!("!{}", render(inner)),
        Node::And(nodes) => group(nodes.iter().map(render), " & "),
        Node::Or(nodes) => group(nodes.iter().map(render), " | "),
        Node::Sequence { first, rest } => {
            let mut out = format!("({}", render(first));
            for (distance, node) in rest {
                out.push(' ');
                out.push_str(&distance_operator(*distance));
                out.push(' ');
                out.push_str(&render(node));
            }
            out.push(')');
            out
        }
    }
}

fn group(parts: impl Iterator<Item = String>, separator: &str) -> String {
    format!("({})", parts.collect::<Vec<_>>().join(separator))
}

/// A tsquery lexeme literal: quoted, with backslashes and quotes escaped, so
/// no character in `text` can act as tsquery syntax.
fn quote_lexeme(text: &str) -> String {
    format!("'{}'", text.replace('\\', "\\\\").replace('\'', "''"))
}

fn distance_operator(distance: u16) -> String {
    if distance == 1 {
        "<->".to_string()
    } else {
        format!("<{distance}>")
    }
}

// ----------------------------------------------------------- highlighting

fn collect_positive(node: &Node, negated: bool, terms: &mut Vec<String>) {
    match node {
        Node::Term { text, .. } if !negated => push_highlight(terms, text),
        Node::Phrase(words) if !negated => push_highlight(terms, &words.join(" ")),
        Node::Term { .. } | Node::Phrase(_) => {}
        Node::Not(inner) => collect_positive(inner, !negated, terms),
        Node::And(nodes) | Node::Or(nodes) => {
            for node in nodes {
                collect_positive(node, negated, terms);
            }
        }
        Node::Sequence { first, rest } => {
            collect_positive(first, negated, terms);
            for (_, node) in rest {
                collect_positive(node, negated, terms);
            }
        }
    }
}

/// Adds `text`, minus surrounding punctuation, unless empty or present.
fn push_highlight(terms: &mut Vec<String>, text: &str) {
    let text = text.trim_matches(|c: char| !c.is_alphanumeric());
    if !text.is_empty() && !terms.iter().any(|t| t == text) {
        terms.push(text.to_string());
    }
}

#[cfg(test)]
#[path = "search_query_tests.rs"]
mod tests;
