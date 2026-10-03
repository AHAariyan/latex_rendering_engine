//! Commutative diagrams: amscd's `\begin{CD} ... \end{CD}`.
//!
//! ```text
//! A @>f>> B          @>a>b>  right arrow, a above, b below    @<a<b<  left
//! @VgVV  @VVhV       @VaVbV  down arrow, a left, b right      @AaAbA  up
//! C @>>k> D          @=  double line   @|  double bar   @.  nothing
//! ```
//!
//! Rows alternate between objects joined by horizontal arrows and rows of
//! vertical arrows standing under the objects. The result is an ordinary
//! array, so layout, hit testing and speech need nothing new.

use super::Parser;
use crate::ast::*;
use crate::error::{Error, Result};
use crate::lexer::Tok;

enum Item {
    Object(Vec<Node>),
    Arrow(Node, bool),
}

impl<'a> Parser<'a> {
    pub(super) fn parse_cd(&mut self, pos: usize) -> Result<Node> {
        let mut rows: Vec<Vec<Vec<Node>>> = Vec::new();
        loop {
            let items = self.cd_row(pos)?;
            rows.push(Self::cd_cells(items));
            match self.lx.advance()? {
                Tok::Cmd("\\") | Tok::Cmd("cr") => continue,
                Tok::Cmd("end") => {
                    let closing = self.lx.raw_group()?;
                    if closing != "CD" {
                        return Err(Error::parse(pos, format!("\\begin{{CD}} closed by \\end{{{closing}}}")));
                    }
                    break;
                }
                Tok::Eof => return Err(Error::parse(pos, "missing \\end{CD}")),
                t => return Err(Error::parse(pos, format!("unexpected {t:?} in CD"))),
            }
        }
        if rows.len() > 1 && rows.last().is_some_and(|r| r.iter().all(|c| c.is_empty())) {
            rows.pop();
        }
        let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(1);
        for r in &mut rows {
            r.resize(ncols, Vec::new());
        }
        let gaps = vec![0.0; rows.len().saturating_sub(1)];
        Ok(Node::Array(Box::new(Array {
            rows,
            cols: vec![ColAlign::Center; ncols],
            cell_style: MathStyle::Display,
            hlines: Vec::new(),
            vlines: Vec::new(),
            row_gaps: gaps,
            pitch: RowPitch::Normal,
            stretch: 1.0,
            outer_sep: false,
        })))
    }

    /// One row: objects and the arrows between or below them.
    fn cd_row(&mut self, pos: usize) -> Result<Vec<Item>> {
        let mut items = Vec::new();
        loop {
            let saved = self.cd_stop.replace('@');
            let object = self.parse_list();
            self.cd_stop = saved;
            let object = object?;
            if !object.is_empty() {
                items.push(Item::Object(object));
            }
            if !matches!(self.lx.peek()?, Tok::Char('@')) {
                return Ok(items);
            }
            self.lx.advance()?;
            items.push(self.cd_arrow(pos)?);
        }
    }

    /// What follows an `@`.
    fn cd_arrow(&mut self, pos: usize) -> Result<Item> {
        let kind = match self.lx.advance()? {
            Tok::Char(c) => c,
            t => return Err(Error::parse(pos, format!("unexpected {t:?} after @ in CD"))),
        };
        Ok(match kind {
            '>' | '<' => {
                let (above, below) = self.cd_labels(kind, pos)?;
                Item::Arrow(
                    Node::XArrow {
                        ch: if kind == '>' { '→' } else { '←' },
                        over: Some(Box::new(Node::Row(above))),
                        under: Some(Box::new(Node::Row(below))),
                    },
                    false,
                )
            }
            'V' | 'A' => {
                let (left, right) = self.cd_labels(kind, pos)?;
                let arrow = Node::SizedDelim {
                    ch: if kind == 'V' { '↓' } else { '↑' },
                    size: 3,
                    atom: AtomType::Ord,
                };
                let label = |body: Vec<Node>| Node::Style {
                    style: MathStyle::Script,
                    body,
                };
                Item::Arrow(
                    Node::Row(vec![
                        Node::Lap {
                            align: Lap::Left,
                            body: Box::new(Node::Row(vec![label(left), Node::Space { mu: 3.0 }])),
                        },
                        arrow,
                        Node::Lap {
                            align: Lap::Right,
                            body: Box::new(Node::Row(vec![Node::Space { mu: 3.0 }, label(right)])),
                        },
                    ]),
                    true,
                )
            }
            '=' => Item::Arrow(
                Node::XArrow {
                    ch: '=',
                    over: None,
                    under: None,
                },
                false,
            ),
            '|' => Item::Arrow(
                Node::SizedDelim {
                    ch: '‖',
                    size: 3,
                    atom: AtomType::Ord,
                },
                true,
            ),
            '.' => Item::Arrow(Node::Row(vec![]), true),
            c => return Err(Error::parse(pos, format!("unknown CD arrow @{c}"))),
        })
    }

    /// `a>b>` after `@>`: the two labels, each ended by the arrow's character.
    fn cd_labels(&mut self, delim: char, pos: usize) -> Result<(Vec<Node>, Vec<Node>)> {
        let mut label = || -> Result<Vec<Node>> {
            let saved = self.cd_stop.replace(delim);
            let body = self.parse_list();
            self.cd_stop = saved;
            let body = body?;
            match self.lx.advance()? {
                Tok::Char(c) if c == delim => Ok(body),
                t => Err(Error::parse(pos, format!("expected `{delim}` in a CD arrow, found {t:?}"))),
            }
        };
        let first = label()?;
        let second = label()?;
        Ok((first, second))
    }

    /// Places a row's items in the grid: objects in even columns, horizontal
    /// arrows between them, vertical arrows under the objects.
    fn cd_cells(items: Vec<Item>) -> Vec<Vec<Node>> {
        let vertical_row = items.iter().all(|i| matches!(i, Item::Arrow(_, true)));
        let mut cells: Vec<Vec<Node>> = Vec::new();
        if vertical_row {
            for (k, item) in items.into_iter().enumerate() {
                if k > 0 {
                    cells.push(Vec::new());
                }
                if let Item::Arrow(node, _) = item {
                    cells.push(vec![node]);
                }
            }
            return cells;
        }
        for item in items {
            match item {
                Item::Object(o) => {
                    if !cells.len().is_multiple_of(2) {
                        cells.push(Vec::new()); // two objects with no arrow between
                    }
                    cells.push(o);
                }
                Item::Arrow(a, _) => {
                    if cells.len().is_multiple_of(2) {
                        cells.push(Vec::new()); // a row that starts with an arrow
                    }
                    cells.push(vec![a]);
                }
            }
        }
        cells
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::Node;
    use crate::parse;

    #[test]
    fn square() {
        let tex = r"\begin{CD} A @>f>> B \\ @VgVV @VVhV \\ C @>>k> D \end{CD}";
        let nodes = parse(tex).unwrap();
        let Node::Array(a) = &nodes[0] else { panic!("{nodes:?}") };
        assert_eq!(a.rows.len(), 3);
        assert_eq!(a.rows[0].len(), 3);
        assert!(matches!(&a.rows[0][1][0], Node::XArrow { ch: '→', .. }));
        assert!(a.rows[1][1].is_empty());
        assert!(crate::render_speech(tex, &crate::Macros::new()).is_ok());
    }

    #[test]
    fn every_arrow_kind_and_errors() {
        assert!(parse(r"\begin{CD} A @<<< B @= C \\ @AAA @| @. \end{CD}").is_ok());
        assert!(parse(r"\begin{CD} A @>f B \end{CD}").is_err());
        assert!(parse(r"\begin{CD} A @? B \end{CD}").is_err());
        assert!(parse(r"\begin{CD} A @>>> B").is_err());
    }
}
