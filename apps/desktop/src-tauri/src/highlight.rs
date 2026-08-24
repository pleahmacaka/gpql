use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub kind: String,
}

struct Grammar {
    parser: Mutex<Parser>,
    query: Query,
}

pub struct Highlighter {
    grammars: HashMap<&'static str, Grammar>,
}

impl Grammar {
    fn new(language: Language, highlights: &str) -> Result<Self, String> {
        let mut parser = Parser::new();

        parser
            .set_language(&language)
            .map_err(|error| error.to_string())?;

        let query = Query::new(&language, highlights).map_err(|error| error.to_string())?;

        return Ok(Grammar {
            parser: Mutex::new(parser),
            query,
        });
    }

    fn tokens(&self, source: &str) -> Vec<Token> {
        let Ok(mut parser) = self.parser.lock() else {
            return Vec::new();
        };
        let Some(tree) = parser.parse(source, None) else {
            return Vec::new();
        };

        let names = self.query.capture_names();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&self.query, tree.root_node(), source.as_bytes());
        let mut tokens: Vec<Token> = Vec::new();

        while let Some(found) = matches.next() {
            for capture in found.captures {
                let span = capture.node.byte_range();
                let kind = names[capture.index as usize];

                if span.start >= span.end {
                    continue;
                }

                tokens.retain(|token| token.start != span.start || token.end != span.end);
                tokens.push(Token {
                    start: span.start,
                    end: span.end,
                    kind: kind.split('.').next().unwrap_or(kind).to_string(),
                });
            }
        }

        tokens.sort_by_key(|token| (token.start, token.end));
        tokens.dedup_by(|later, earlier| later.start < earlier.end);

        return tokens;
    }
}

impl Highlighter {
    pub fn new() -> Result<Self, String> {
        let mut grammars = HashMap::new();

        grammars.insert(
            "sql",
            Grammar::new(
                tree_sitter_sequel::LANGUAGE.into(),
                tree_sitter_sequel::HIGHLIGHTS_QUERY,
            )?,
        );

        grammars.insert(
            "cypher",
            Grammar::new(
                tree_sitter_cypher::LANGUAGE.into(),
                tree_sitter_cypher::HIGHLIGHTS_QUERY,
            )?,
        );

        return Ok(Highlighter { grammars });
    }

    pub fn tokens(&self, dialect: &str, source: &str) -> Vec<Token> {
        let grammar = self
            .grammars
            .get(dialect)
            .or_else(|| self.grammars.get("sql"));

        return grammar.map(|found| found.tokens(source)).unwrap_or_default();
    }
}
