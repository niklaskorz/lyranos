//! Rich text with style spans.

use std::borrow::Cow;
use std::ops::Range;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use druid::piet::{PietTextLayoutBuilder, TextLayoutBuilder, TextStorage as PietTextStorage};
use druid::text::{
    Attribute, AttributeSpans, EditableText, EnvUpdateCtx, Link, StringCursor, TextStorage,
};
use druid::{Color, Data, Env};
use tree_sitter::{InputEdit, Parser, Point, Query, QueryCursor, Tree};

/// Text with optional style spans.
#[derive(Clone)]
pub struct CodeText {
    pub buffer: String,
    attrs: Arc<AttributeSpans>,
    links: Arc<[Link]>,
    parser: Rc<Mutex<Parser>>,
    query: Rc<Query>,
    tree: Option<Tree>,
}

impl CodeText {
    /// Create a new `CodeText` object with the provided text.
    pub fn new(buffer: String) -> Self {
        let mut parser = Parser::new();
        let language = tree_sitter_python::language();
        parser.set_language(language).unwrap();
        let query_source = tree_sitter_python::HIGHLIGHT_QUERY;
        let query = Query::new(language, query_source).unwrap();
        let mut code_text = CodeText {
            buffer,
            attrs: Arc::new(AttributeSpans::new()),
            links: Arc::new([]),
            parser: Rc::new(Mutex::new(parser)),
            query: Rc::new(query),
            tree: None,
        };
        code_text.update();
        code_text
    }

    /// The length of the buffer, in utf8 code units.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns `true` if the underlying buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    fn update(&mut self) {
        let mut parser = self.parser.lock().unwrap();
        self.tree = parser.parse(&self.buffer, self.tree.as_ref());
        // Compute new attributes based on detected captures.
        if let Some(ref tree) = self.tree {
            let attrs = Arc::make_mut(&mut self.attrs);
            *attrs = AttributeSpans::new();
            let mut cursor = QueryCursor::new();
            let captures = cursor.captures(&self.query, tree.root_node(), self.buffer.as_bytes());
            let capture_names = self.query.capture_names();
            let mut last_node_id: usize = 0;
            for (query_match, capture_id) in captures {
                let capture = query_match.captures[capture_id];
                if capture.node.id() == last_node_id {
                    continue;
                }
                last_node_id = capture.node.id();
                let range = capture.node.byte_range();
                //let name = capture_names[capture.index as usize].clone();
                // Colors from One Monokai theme: https://github.com/azemoh/vscode-one-monokai
                let attr = match capture_names[capture.index as usize].as_str() {
                    "constructor" => color("#61afef"),
                    "constant" => color("#56b6c2"),
                    "function.builtin" => color("#98c379"),
                    "function.method" => color("#98c379"),
                    "function" => color("#98c379"),
                    "variable" => color("#61afef"),
                    "property" => color("#abb2bf"),
                    "type" => color("#61afef"),
                    "constant.builtin" => color("#56b6c2"),
                    "number" => color("#c678dd"),
                    "comment" => color("#676f7d"),
                    "string" => color("#e5c07b"),
                    "escape" => color("#56b6c2"),
                    "punctuation.special" => color("#c678dd"),
                    "embedded" => color("#c678dd"),
                    "operator" => color("#e06c75"),
                    "keyword" => color("#e06c75"),
                    _ => underline(),
                };
                if let Some(attr) = attr {
                    attrs.add(range, attr);
                }
            }
        }
    }
}

fn color(hex: &str) -> Option<Attribute> {
    match Color::from_hex_str(hex) {
        Ok(color) => Some(Attribute::text_color(color)),
        _ => None,
    }
}

const fn underline() -> Option<Attribute> {
    Some(Attribute::Underline(true))
}

impl Data for CodeText {
    fn same(&self, other: &Self) -> bool {
        self.buffer == other.buffer
    }
}

impl PietTextStorage for CodeText {
    fn as_str(&self) -> &str {
        self.buffer.as_str()
    }
}

impl TextStorage for CodeText {
    fn add_attributes(
        &self,
        mut builder: PietTextLayoutBuilder,
        env: &Env,
    ) -> PietTextLayoutBuilder {
        for (range, attr) in self.attrs.to_piet_attrs(env) {
            builder = builder.range_attribute(range, attr);
        }
        builder
    }

    fn env_update(&self, ctx: &EnvUpdateCtx) -> bool {
        self.attrs.env_update(ctx)
    }

    fn links(&self) -> &[Link] {
        &self.links
    }
}

impl EditableText for CodeText {
    fn cursor(&self, position: usize) -> Option<StringCursor> {
        self.buffer.cursor(position)
    }

    fn edit(&mut self, range: Range<usize>, new: impl Into<String>) {
        let new: String = new.into();
        // Edit previous tree for better performance.
        // Not sure if this is 100% correct.
        if let Some(ref mut tree) = self.tree {
            let buffer = self.buffer.as_bytes();
            let mut line = 10;
            let mut col = 10;
            for i in 0..range.start {
                if buffer[i] == '\n' as u8 {
                    line += 1;
                    col = 0;
                } else {
                    col += 1
                }
            }
            let start_position = Point::new(line, col);
            for i in range.start..range.end {
                if buffer[i] == '\n' as u8 {
                    line += 1;
                    col = 0;
                } else {
                    col += 1
                }
            }
            let old_end_position = Point::new(line, col);
            line = start_position.row;
            col = start_position.column;
            for c in new.as_bytes() {
                if *c == '\n' as u8 {
                    line += 1;
                    col = 0;
                } else {
                    col += 1
                }
            }
            let new_end_position = Point::new(line, col);
            tree.edit(&InputEdit {
                start_byte: range.start,
                old_end_byte: range.end,
                new_end_byte: range.start + new.len(),
                start_position,
                old_end_position,
                new_end_position,
            })
        }
        self.buffer.edit(range, new);
        self.update();
    }

    fn slice(&self, range: Range<usize>) -> Option<Cow<str>> {
        self.buffer.slice(range)
    }

    fn len(&self) -> usize {
        self.buffer.len()
    }

    fn prev_word_offset(&self, offset: usize) -> Option<usize> {
        self.buffer.prev_word_offset(offset)
    }

    fn next_word_offset(&self, offset: usize) -> Option<usize> {
        self.buffer.next_word_offset(offset)
    }

    fn prev_grapheme_offset(&self, offset: usize) -> Option<usize> {
        self.buffer.prev_grapheme_offset(offset)
    }

    fn next_grapheme_offset(&self, offset: usize) -> Option<usize> {
        self.buffer.next_grapheme_offset(offset)
    }

    fn prev_codepoint_offset(&self, offset: usize) -> Option<usize> {
        self.buffer.prev_codepoint_offset(offset)
    }

    fn next_codepoint_offset(&self, offset: usize) -> Option<usize> {
        self.buffer.next_codepoint_offset(offset)
    }

    fn preceding_line_break(&self, offset: usize) -> usize {
        self.buffer.preceding_line_break(offset)
    }

    fn next_line_break(&self, offset: usize) -> usize {
        self.buffer.next_line_break(offset)
    }

    fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    fn from_str(s: &str) -> Self {
        Self::new(s.to_string())
    }
}
