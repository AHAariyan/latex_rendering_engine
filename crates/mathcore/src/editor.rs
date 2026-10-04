//! A structural math editor: the model behind a math input field.
//!
//! The formula is a tree of atoms (symbols, fractions, roots, scripts,
//! bracket pairs), and the cursor sits between two atoms of one list in it.
//! Typing follows the conventions of the established math fields: `/` turns
//! the term before the cursor into a numerator, `^` and `_` open a script,
//! `(` opens a pair whose closing bracket is typed over, `\` starts a command
//! name, and names like `sqrt`, `pi` or `sin` become what they name as soon
//! as they are typed. Arrows walk into and out of structures; Backspace at
//! the start of a slot takes the structure apart instead of deleting it.
//!
//! The editor renders through the ordinary engine: it writes the formula as
//! TeX, with a placeholder box in every empty slot, records the source
//! offset of every cursor position while doing so, and lays the TeX out with
//! hit testing. The caret, the selection and a click all come from the
//! source regions of that layout, so they always line up with what is drawn.
//! Platform views only forward keys and taps and draw the result.

use crate::display::{DisplayList, Region};
use crate::font::MathFont;
use crate::layout::RenderOptions;
use crate::Result;

/// One element of an editable list.
#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    /// A symbol as TeX: `x`, `+`, `\alpha`, `\sin`; or an opaque piece of TeX
    /// the editor does not take apart (a matrix), edited as one unit.
    Sym(String),
    Frac(Vec<Atom>, Vec<Atom>),
    Sqrt(Vec<Atom>),
    /// An nth root: index, radicand.
    Root(Vec<Atom>, Vec<Atom>),
    /// Scripts on the atom before it (or on nothing, at the start of a list).
    Scripts {
        sub: Option<Vec<Atom>>,
        sup: Option<Vec<Atom>>,
    },
    /// A pair of growing delimiters, as TeX: `(` `)`, `\{` `\}`, `|` `|`.
    Group {
        open: String,
        close: String,
        body: Vec<Atom>,
    },
}

/// Slot numbers: Frac 0 numerator, 1 denominator; Sqrt 0; Root 0 index,
/// 1 radicand; Scripts 0 subscript, 1 superscript; Group 0.
impl Atom {
    /// The slots this atom has, in reading order.
    fn slots(&self) -> Vec<usize> {
        match self {
            Atom::Sym(_) => vec![],
            Atom::Frac(..) | Atom::Root(..) => vec![0, 1],
            Atom::Sqrt(_) | Atom::Group { .. } => vec![0],
            Atom::Scripts { sub, sup } => {
                let mut v = vec![];
                if sub.is_some() {
                    v.push(0);
                }
                if sup.is_some() {
                    v.push(1);
                }
                v
            }
        }
    }

    fn slot(&self, k: usize) -> &Vec<Atom> {
        match (self, k) {
            (Atom::Frac(n, _), 0) | (Atom::Root(n, _), 0) | (Atom::Sqrt(n), 0) => n,
            (Atom::Frac(_, d), 1) | (Atom::Root(_, d), 1) => d,
            (Atom::Group { body, .. }, 0) => body,
            (Atom::Scripts { sub: Some(s), .. }, 0) => s,
            (Atom::Scripts { sup: Some(s), .. }, 1) => s,
            _ => panic!("no slot {k} in {self:?}"),
        }
    }

    fn slot_mut(&mut self, k: usize) -> &mut Vec<Atom> {
        match (self, k) {
            (Atom::Frac(n, _), 0) | (Atom::Root(n, _), 0) | (Atom::Sqrt(n), 0) => n,
            (Atom::Frac(_, d), 1) | (Atom::Root(_, d), 1) => d,
            (Atom::Group { body, .. }, 0) => body,
            (Atom::Scripts { sub: Some(s), .. }, 0) => s,
            (Atom::Scripts { sup: Some(s), .. }, 1) => s,
            (a, k) => panic!("no slot {k} in {a:?}"),
        }
    }

    fn is_empty_structure(&self) -> bool {
        !matches!(self, Atom::Sym(_)) && self.slots().iter().all(|&k| self.slot(k).is_empty())
    }

    /// Ends a term: `/` takes the atoms before the cursor back to one of these.
    fn is_operator(&self) -> bool {
        let Atom::Sym(s) = self else { return false };
        matches!(
            s.as_str(),
            "+" | "-"
                | "="
                | "<"
                | ">"
                | ","
                | ";"
                | ":"
                | "!"
                | "\\pm"
                | "\\mp"
                | "\\cdot"
                | "\\times"
                | "\\div"
                | "\\le"
                | "\\ge"
                | "\\ne"
                | "\\leq"
                | "\\geq"
                | "\\neq"
                | "\\approx"
                | "\\equiv"
                | "\\to"
                | "\\in"
                | "\\notin"
                | "\\subset"
                | "\\subseteq"
                | "\\cup"
                | "\\cap"
                | "\\implies"
                | "\\iff"
                | "\\rightarrow"
                | "\\Rightarrow"
                | "\\sim"
                | "\\propto"
                | "\\sum"
                | "\\int"
                | "\\prod"
                | "\\lim"
        )
    }
}

/// A place between two atoms: which list (the path of atom index and slot
/// from the root), and the index in it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cursor {
    pub path: Vec<(usize, usize)>,
    pub pos: usize,
}

/// Keys an editor understands. Text arrives through `Editor::type_text`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    Backspace,
    Delete,
    /// Shift+arrow: extend the selection.
    SelectLeft,
    SelectRight,
    SelectAll,
    /// Enter or Tab: finish a `\command` being typed, else leave the slot.
    Enter,
    Undo,
    Redo,
}

impl Key {
    /// A key by its DOM name (`ArrowLeft`, `Backspace`, `Home`...), with
    /// Shift and Ctrl/Cmd: Shift+arrow selects, Cmd+A selects all, Cmd+Z
    /// undoes, Cmd+Shift+Z or Cmd+Y redoes. `None` for keys the editor does
    /// not handle (letters arrive as text).
    pub fn from_name(name: &str, shift: bool, command: bool) -> Option<Key> {
        Some(match (name, shift, command) {
            ("ArrowLeft", true, _) => Key::SelectLeft,
            ("ArrowRight", true, _) => Key::SelectRight,
            ("ArrowLeft", false, _) => Key::Left,
            ("ArrowRight", false, _) => Key::Right,
            ("ArrowUp", ..) => Key::Up,
            ("ArrowDown", ..) => Key::Down,
            ("Home", ..) => Key::Home,
            ("End", ..) => Key::End,
            ("Backspace", ..) => Key::Backspace,
            ("Delete", ..) => Key::Delete,
            ("Enter" | "Tab", ..) => Key::Enter,
            ("a" | "A", _, true) => Key::SelectAll,
            ("z" | "Z", true, true) | ("y" | "Y", _, true) => Key::Redo,
            ("z" | "Z", false, true) => Key::Undo,
            _ => return None,
        })
    }
}

/// A rectangle in the layout's pixel space, y down.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// The editor drawn: the formula, where the caret goes, what is selected.
#[derive(Debug, Clone)]
pub struct EditorLayout {
    pub display: DisplayList,
    pub caret: Rect,
    /// One rectangle per selected atom; empty without a selection.
    pub selection: Vec<Rect>,
    positions: Vec<(usize, Cursor)>,
}

#[derive(Debug, Clone, Default)]
pub struct Editor {
    root: Vec<Atom>,
    cur: Cursor,
    /// The other end of the selection, in the same list as `cur`.
    anchor: Option<Cursor>,
    /// A `\command` being typed at the cursor.
    command: Option<String>,
    undo: Vec<(Vec<Atom>, Cursor)>,
    redo: Vec<(Vec<Atom>, Cursor)>,
}

/// Names that turn into what they name as soon as they are typed.
const AUTO: &[&str] = &[
    "sqrt", "nthroot", "frac", "sum", "prod", "int", "oint", "lim", "infty", "infinity", "pi", "theta", "alpha", "beta", "gamma", "delta",
    "epsilon", "lambda", "mu", "sigma", "omega", "phi", "rho", "tau", "Delta", "Omega", "Sigma", "Pi", "sin", "cos", "tan", "cot", "sec",
    "csc", "arcsin", "arccos", "arctan", "sinh", "cosh", "tanh", "log", "ln", "exp", "det", "max", "min", "pm", "cdot", "times", "div",
    "approx", "cup", "cap", "forall", "exists", "partial", "nabla",
];

impl Editor {
    pub fn new() -> Self {
        Self::default()
    }

    /// An editor holding `tex`, with the cursor at the end. Structures the
    /// editor knows are taken apart for editing; anything else (a matrix, an
    /// environment) becomes one opaque atom that is kept as written.
    pub fn from_tex(tex: &str) -> Self {
        let mut e = Editor::new();
        e.root = TexReader::new(tex).list(None);
        e.cur.pos = e.root.len();
        e
    }

    /// The formula as TeX, with nothing of the editing in it.
    pub fn tex(&self) -> String {
        let mut out = String::new();
        write_list(&self.root, &mut out);
        out.trim().to_string()
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_empty() && self.command.is_none()
    }

    pub fn cursor(&self) -> &Cursor {
        &self.cur
    }

    pub fn has_selection(&self) -> bool {
        let (a, b) = self.selection_range();
        a != b
    }

    /// The selection as TeX, for copying.
    pub fn selected_tex(&self) -> String {
        let (a, b) = self.selection_range();
        let mut out = String::new();
        write_list(&self.list()[a..b], &mut out);
        out.trim().to_string()
    }

    /// The cursor's path leads to a list and its position is inside it.
    pub fn cursor_is_valid(&self) -> bool {
        let mut list = &self.root;
        for &(i, k) in &self.cur.path {
            match list.get(i) {
                Some(a) if a.slots().contains(&k) => list = a.slot(k),
                _ => return false,
            }
        }
        self.cur.pos <= list.len()
    }

    /// What a screen reader says for the cursor's place: the slot it is in
    /// and what it is after ("denominator, 2"; "superscript, blank"), in
    /// the language of `opts`. Hosts announce it after each key.
    pub fn describe(&self, opts: &crate::a11y::SpeechOptions) -> String {
        let lang = opts.language;
        let tr = |s: &str| crate::speech_lang::translate(lang, s);
        let label = self.cur.path.last().map(|&(i, k)| {
            let parent = list_at(&self.root, &self.cur.path[..self.cur.path.len() - 1]);
            tr(match (&parent[i], k) {
                (Atom::Frac(..), 0) => "numerator",
                (Atom::Frac(..), _) => "denominator",
                (Atom::Root(..), 0) => "index",
                (Atom::Root(..), _) | (Atom::Sqrt(_), _) => "radicand",
                (Atom::Scripts { .. }, 0) => "subscript",
                (Atom::Scripts { .. }, _) => "superscript",
                _ => "contents",
            })
        });
        let list = self.list();
        let neighbour = if let Some(name) = &self.command {
            format!("\\{name}")
        } else if self.has_selection() {
            let (a, b) = self.selection_range();
            speak(&list[a..b], opts)
        } else if list.is_empty() {
            tr("blank")
        } else if self.cur.pos > 0 {
            // Scripts are read with their base.
            let from = if matches!(list[self.cur.pos - 1], Atom::Scripts { .. }) {
                self.cur.pos.saturating_sub(2)
            } else {
                self.cur.pos - 1
            };
            speak(&list[from..self.cur.pos], opts)
        } else {
            String::new()
        };
        match label {
            Some(l) if neighbour.is_empty() => l,
            Some(l) => format!("{l}, {neighbour}"),
            None => neighbour,
        }
    }

    /// The whole formula read aloud.
    pub fn speech(&self, opts: &crate::a11y::SpeechOptions) -> String {
        speak(&self.root, opts)
    }

    // ---- the tree ----

    fn list(&self) -> &Vec<Atom> {
        list_at(&self.root, &self.cur.path)
    }

    fn list_mut(&mut self) -> &mut Vec<Atom> {
        list_at_mut(&mut self.root, &self.cur.path)
    }

    /// The selected range of the cursor's list. Both ends of a selection
    /// are always in one list; an anchor left elsewhere selects nothing.
    fn selection_range(&self) -> (usize, usize) {
        let len = self.list().len();
        match &self.anchor {
            Some(a) if a.path == self.cur.path => (a.pos.min(self.cur.pos).min(len), a.pos.max(self.cur.pos).min(len)),
            _ => (self.cur.pos, self.cur.pos),
        }
    }

    fn checkpoint(&mut self) {
        self.undo.push((self.root.clone(), self.cur.clone()));
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Removes the selection and returns it.
    fn take_selection(&mut self) -> Vec<Atom> {
        let (a, b) = self.selection_range();
        self.anchor = None;
        self.cur.pos = a;
        self.list_mut().drain(a..b).collect()
    }

    fn insert_atom(&mut self, atom: Atom) {
        let pos = self.cur.pos;
        self.list_mut().insert(pos, atom);
        self.cur.pos += 1;
    }

    /// Puts the cursor at the start of `slot` of the atom at `index` in the
    /// current list.
    fn enter(&mut self, index: usize, slot: usize, at_end: bool) {
        let len = self.list()[index].slot(slot).len();
        self.cur.path.push((index, slot));
        self.cur.pos = if at_end { len } else { 0 };
    }

    // ---- typing ----

    /// Types text at the cursor, character by character, with the editor's
    /// conventions for `/ ^ _ ( ) [ ] { } | \`.
    pub fn type_text(&mut self, text: &str) {
        for c in text.chars() {
            self.type_char(c);
        }
    }

    fn type_char(&mut self, c: char) {
        if let Some(name) = &mut self.command {
            if c.is_ascii_alphabetic() {
                name.push(c);
                return;
            }
            self.finish_command();
            if c == ' ' {
                return;
            }
        }
        self.checkpoint();
        match c {
            '\\' => {
                self.take_selection();
                self.command = Some(String::new());
            }
            ' ' => {}
            '/' => self.fraction(),
            '^' => self.script(true),
            '_' => self.script(false),
            '(' => self.group("(", ")"),
            '[' => self.group("[", "]"),
            '{' => self.group("\\{", "\\}"),
            '|' if !self.close_group("|") => self.group("|", "|"),
            '|' => {}
            ')' | ']' | '}' => {
                let close = match c {
                    ')' => ")",
                    ']' => "]",
                    _ => "\\}",
                };
                if !self.close_group(close) {
                    self.take_selection();
                    self.insert_atom(Atom::Sym(close.into()));
                }
            }
            _ => {
                self.take_selection();
                let tex = match c {
                    '*' => "\\cdot".to_string(),
                    '#' | '$' | '%' | '&' => format!("\\{c}"),
                    '~' => "\\sim".to_string(),
                    c => c.to_string(),
                };
                self.insert_atom(Atom::Sym(tex));
                if c.is_ascii_alphabetic() {
                    self.auto_command();
                }
            }
        }
    }

    /// Inserts TeX at the cursor as structure (pasted `\frac{a}{b}` stays a
    /// fraction), replacing the selection.
    pub fn insert_tex(&mut self, tex: &str) {
        self.finish_command();
        self.checkpoint();
        self.take_selection();
        let atoms = TexReader::new(tex).list(None);
        let n = atoms.len();
        let pos = self.cur.pos;
        self.list_mut().splice(pos..pos, atoms);
        self.cur.pos += n;
    }

    /// Runs a command by name, as typing `\name` and a space would: `frac`,
    /// `sqrt`, `nthroot`, a symbol (`alpha`, `le`), a function (`sin`).
    pub fn command(&mut self, name: &str) {
        self.checkpoint();
        self.apply_command(name);
    }

    fn apply_command(&mut self, name: &str) {
        match name {
            "frac" | "dfrac" | "tfrac" => self.fraction_with(Vec::new()),
            "sqrt" => {
                let body = self.take_selection();
                self.insert_atom(Atom::Sqrt(body));
                let i = self.cur.pos - 1;
                self.cur.pos = i;
                self.enter(i, 0, true);
            }
            "nthroot" | "root" => {
                let body = self.take_selection();
                self.insert_atom(Atom::Root(Vec::new(), body));
                let i = self.cur.pos - 1;
                self.cur.pos = i;
                self.enter(i, 0, false);
            }
            "sup" | "superscript" => self.script(true),
            "sub" | "subscript" => self.script(false),
            "abs" => self.group("|", "|"),
            "norm" => self.group("\\|", "\\|"),
            "paren" | "left" => self.group("(", ")"),
            "infinity" => self.insert_atom(Atom::Sym("\\infty".into())),
            "" => {}
            _ => {
                self.take_selection();
                // A name the engine does not know is typed out as letters
                // rather than becoming a formula that cannot be drawn.
                if crate::parse(&format!("\\{name}")).is_ok() {
                    self.insert_atom(Atom::Sym(format!("\\{name}")));
                } else {
                    for ch in name.chars() {
                        self.insert_atom(Atom::Sym(ch.to_string()));
                    }
                }
            }
        }
    }

    fn finish_command(&mut self) {
        if let Some(name) = self.command.take() {
            self.checkpoint();
            self.apply_command(&name);
        }
    }

    /// `sqrt`, `pi`, `sin`... typed as letters become the command.
    fn auto_command(&mut self) {
        let end = self.cur.pos;
        // `\cos` and an `h` make `\cosh`.
        if end >= 2 {
            if let (Atom::Sym(prev), Atom::Sym(c)) = (&self.list()[end - 2], &self.list()[end - 1]) {
                if let Some(name) = prev.strip_prefix('\\') {
                    let longer = format!("{name}{c}");
                    if AUTO.contains(&longer.as_str()) {
                        self.list_mut().drain(end - 2..end);
                        self.cur.pos = end - 2;
                        self.apply_command(&longer);
                        return;
                    }
                }
            }
        }
        let list = self.list();
        let mut start = end;
        while start > 0 {
            match &list[start - 1] {
                Atom::Sym(s) if s.len() == 1 && s.chars().all(|c| c.is_ascii_alphabetic()) => start -= 1,
                _ => break,
            }
        }
        let word: String = list[start..end]
            .iter()
            .map(|a| if let Atom::Sym(s) = a { s.as_str() } else { "" })
            .collect();
        // The longest name the word ends with: "arcsin", not "sin".
        let Some(name) = AUTO.iter().filter(|n| word.ends_with(**n)).max_by_key(|n| n.len()) else {
            return;
        };
        let from = end - name.len();
        self.list_mut().drain(from..end);
        self.cur.pos = from;
        self.apply_command(name);
    }

    /// `/`: the term before the cursor (back to an operator) becomes the
    /// numerator and the cursor goes to the denominator; with nothing
    /// before, the cursor goes to an empty numerator.
    fn fraction(&mut self) {
        if self.has_selection() {
            let num = self.take_selection();
            self.fraction_with(num);
            return;
        }
        let list = self.list();
        let mut start = self.cur.pos;
        while start > 0 && !list[start - 1].is_operator() {
            start -= 1;
        }
        let end = self.cur.pos;
        let num: Vec<Atom> = self.list_mut().drain(start..end).collect();
        self.cur.pos = start;
        self.fraction_with(num);
    }

    fn fraction_with(&mut self, num: Vec<Atom>) {
        let filled = !num.is_empty();
        self.insert_atom(Atom::Frac(num, Vec::new()));
        let i = self.cur.pos - 1;
        self.cur.pos = i;
        self.enter(i, if filled { 1 } else { 0 }, false);
    }

    /// `^` or `_`: into the script of the atom before the cursor, making it
    /// if needed.
    fn script(&mut self, sup: bool) {
        self.take_selection();
        let slot = if sup { 1 } else { 0 };
        let pos = self.cur.pos;
        // Scripts already after the cursor's atom: x_1|^2 or x|^2.
        let at = if pos > 0 && matches!(self.list()[pos - 1], Atom::Scripts { .. }) {
            Some(pos - 1)
        } else if matches!(self.list().get(pos), Some(Atom::Scripts { .. })) {
            Some(pos)
        } else {
            None
        };
        let i = match at {
            Some(i) => {
                if let Atom::Scripts { sub, sup: sp } = &mut self.list_mut()[i] {
                    let s = if slot == 1 { sp } else { sub };
                    s.get_or_insert_with(Vec::new);
                }
                i
            }
            None => {
                let (sub, sp) = if sup { (None, Some(Vec::new())) } else { (Some(Vec::new()), None) };
                self.insert_atom(Atom::Scripts { sub, sup: sp });
                self.cur.pos - 1
            }
        };
        self.cur.pos = i;
        self.enter(i, slot, true);
    }

    fn group(&mut self, open: &str, close: &str) {
        let body = self.take_selection();
        let filled = !body.is_empty();
        self.insert_atom(Atom::Group {
            open: open.into(),
            close: close.into(),
            body,
        });
        let i = self.cur.pos - 1;
        if filled {
            return;
        }
        self.cur.pos = i;
        self.enter(i, 0, false);
    }

    /// A closing bracket typed inside a pair it closes moves out of the
    /// pair; what was after the cursor in it follows the pair.
    fn close_group(&mut self, close: &str) -> bool {
        let Some(&(i, _)) = self.cur.path.last() else { return false };
        let parent = &self.cur.path[..self.cur.path.len() - 1];
        let matches = matches!(&list_at(&self.root, parent)[i], Atom::Group { close: c, .. } if c == close);
        if !matches {
            return false;
        }
        let pos = self.cur.pos;
        let rest: Vec<Atom> = self.list_mut().drain(pos..).collect();
        self.cur.path.pop();
        self.cur.pos = i + 1;
        let at = self.cur.pos;
        self.list_mut().splice(at..at, rest);
        true
    }

    // ---- keys ----

    pub fn key(&mut self, key: Key) {
        if self.command.is_some() {
            match key {
                Key::Backspace => {
                    let name = self.command.as_mut().unwrap();
                    if name.pop().is_none() {
                        self.command = None;
                    }
                    return;
                }
                Key::Enter => {
                    self.finish_command();
                    return;
                }
                _ => self.finish_command(),
            }
        }
        match key {
            Key::Left | Key::Right => {
                if self.has_selection() {
                    let (a, b) = self.selection_range();
                    self.cur.pos = if key == Key::Left { a } else { b };
                    self.anchor = None;
                } else {
                    self.anchor = None;
                    if key == Key::Left {
                        self.left()
                    } else {
                        self.right()
                    }
                }
            }
            Key::Up => self.vertical(true),
            Key::Down => self.vertical(false),
            Key::Home => {
                self.anchor = None;
                self.cur.pos = 0;
            }
            Key::End => {
                self.anchor = None;
                self.cur.pos = self.list().len();
            }
            Key::SelectLeft | Key::SelectRight => self.extend(key == Key::SelectRight),
            Key::SelectAll => {
                self.cur = Cursor {
                    path: vec![],
                    pos: self.root.len(),
                };
                self.anchor = Some(Cursor { path: vec![], pos: 0 });
            }
            Key::Backspace => {
                self.checkpoint();
                self.backspace();
            }
            Key::Delete => {
                self.checkpoint();
                self.delete();
            }
            Key::Enter => {
                self.anchor = None;
                self.exit_slot();
            }
            Key::Undo => {
                if let Some((root, cur)) = self.undo.pop() {
                    self.redo
                        .push((std::mem::replace(&mut self.root, root), std::mem::replace(&mut self.cur, cur)));
                    self.anchor = None;
                }
            }
            Key::Redo => {
                if let Some((root, cur)) = self.redo.pop() {
                    self.undo
                        .push((std::mem::replace(&mut self.root, root), std::mem::replace(&mut self.cur, cur)));
                    self.anchor = None;
                }
            }
        }
    }

    fn right(&mut self) {
        let pos = self.cur.pos;
        if let Some(atom) = self.list().get(pos) {
            match atom.slots().first() {
                Some(&slot) => self.enter(pos, slot, false),
                None => self.cur.pos += 1,
            }
            return;
        }
        self.leave(true);
    }

    fn left(&mut self) {
        let pos = self.cur.pos;
        if pos > 0 {
            match self.list()[pos - 1].slots().last() {
                Some(&slot) => {
                    self.cur.pos = pos - 1;
                    self.enter(pos - 1, slot, true)
                }
                None => self.cur.pos -= 1,
            }
            return;
        }
        self.leave(false);
    }

    /// At the end (or start) of a slot: on to the next (or previous) slot of
    /// the same structure, else out of it.
    fn leave(&mut self, forward: bool) {
        let Some((i, slot)) = self.cur.path.pop() else { return };
        let slots = list_at(&self.root, &self.cur.path)[i].slots();
        let k = slots.iter().position(|&s| s == slot).unwrap_or(0);
        let next = if forward {
            slots.get(k + 1)
        } else {
            k.checked_sub(1).and_then(|k| slots.get(k))
        };
        match next {
            Some(&s) => {
                self.cur.pos = i;
                self.enter(i, s, !forward);
            }
            None => self.cur.pos = if forward { i + 1 } else { i },
        }
    }

    /// Enter in a slot: out of the structure, after it.
    fn exit_slot(&mut self) {
        if let Some((i, _)) = self.cur.path.pop() {
            self.cur.pos = i + 1;
        }
    }

    /// Up and Down: numerator and denominator, superscript and subscript.
    fn vertical(&mut self, up: bool) {
        self.anchor = None;
        // Innermost structure first: from the denominator up to the numerator.
        for depth in (0..self.cur.path.len()).rev() {
            let (i, slot) = self.cur.path[depth];
            let atom = &list_at(&self.root, &self.cur.path[..depth])[i];
            let target = match (atom, slot, up) {
                (Atom::Frac(..), 1, true) => Some(0),
                (Atom::Frac(..), 0, false) => Some(1),
                (Atom::Scripts { sup: Some(_), .. }, 0, true) => Some(1),
                (Atom::Scripts { sub: Some(_), .. }, 1, false) => Some(0),
                _ => None,
            };
            if let Some(t) = target {
                let ratio = self.cur.pos;
                self.cur.path.truncate(depth);
                self.cur.pos = i;
                self.enter(i, t, false);
                self.cur.pos = ratio.min(self.list().len());
                return;
            }
        }
        // Beside a fraction: into its numerator or denominator.
        let pos = self.cur.pos;
        let list = self.list();
        let beside = [pos.checked_sub(1), Some(pos)]
            .into_iter()
            .flatten()
            .find(|&i| matches!(list.get(i), Some(Atom::Frac(..))));
        if let Some(i) = beside {
            let at_end = i < pos;
            self.cur.pos = i;
            self.enter(i, if up { 0 } else { 1 }, at_end);
        }
    }

    fn extend(&mut self, forward: bool) {
        if self.anchor.is_none() {
            self.anchor = Some(self.cur.clone());
        }
        let len = self.list().len();
        let at_edge = if forward { self.cur.pos == len } else { self.cur.pos == 0 };
        if !at_edge {
            if forward {
                self.cur.pos += 1
            } else {
                self.cur.pos -= 1
            }
            return;
        }
        // At the edge of a slot: the selection grows to the whole structure.
        if let Some((i, _)) = self.cur.path.pop() {
            let (a, b) = if forward { (i, i + 1) } else { (i + 1, i) };
            self.anchor = Some(Cursor {
                path: self.cur.path.clone(),
                pos: a,
            });
            self.cur.pos = b;
        }
    }

    fn backspace(&mut self) {
        if self.has_selection() {
            self.take_selection();
            return;
        }
        self.anchor = None;
        let pos = self.cur.pos;
        if pos > 0 {
            let atom = &self.list()[pos - 1];
            if atom.slots().is_empty() || atom.is_empty_structure() {
                self.list_mut().remove(pos - 1);
                self.cur.pos -= 1;
            } else {
                // Into the structure, to delete from its end.
                let slot = *atom.slots().last().unwrap();
                self.cur.pos = pos - 1;
                self.enter(pos - 1, slot, true);
            }
            return;
        }
        let Some(&(i, slot)) = self.cur.path.last() else { return };
        let parent_path = self.cur.path[..self.cur.path.len() - 1].to_vec();
        let parent = &list_at(&self.root, &parent_path)[i];
        let slots = parent.slots();
        let k = slots.iter().position(|&s| s == slot).unwrap_or(0);
        if k > 0 && !parent.is_empty_structure() {
            // From the start of the denominator to the end of the numerator.
            self.cur.path.pop();
            self.cur.pos = i;
            self.enter(i, slots[k - 1], true);
            return;
        }
        self.unwrap(&parent_path, i);
    }

    /// Replaces the structure at `i` with the contents of its slots, cursor
    /// where the structure began: Backspace at the start of a fraction's
    /// numerator leaves "a b" where "a/b" was.
    fn unwrap(&mut self, parent_path: &[(usize, usize)], i: usize) {
        let list = list_at_mut(&mut self.root, parent_path);
        let atom = list.remove(i);
        let mut contents: Vec<Atom> = vec![];
        for k in atom.slots() {
            contents.extend(atom.slot(k).iter().cloned());
        }
        let n = contents.len();
        list.splice(i..i, contents);
        let _ = n;
        self.cur = Cursor {
            path: parent_path.to_vec(),
            pos: i,
        };
    }

    fn delete(&mut self) {
        if self.has_selection() {
            self.take_selection();
            return;
        }
        let pos = self.cur.pos;
        if let Some(atom) = self.list().get(pos) {
            if atom.slots().is_empty() || atom.is_empty_structure() {
                self.list_mut().remove(pos);
            } else {
                let slot = atom.slots()[0];
                self.enter(pos, slot, false);
            }
            return;
        }
        // At the end of a slot: an empty structure goes; otherwise step out.
        if let Some(&(i, _)) = self.cur.path.last() {
            let parent_path = self.cur.path[..self.cur.path.len() - 1].to_vec();
            if list_at(&self.root, &parent_path)[i].is_empty_structure() {
                self.unwrap(&parent_path, i);
            } else {
                self.leave(true);
            }
        }
    }

    // ---- drawing ----

    /// The formula as the editor draws it: TeX with a placeholder in every
    /// empty slot and the command being typed, plus the source offset of the
    /// caret and of every cursor position.
    pub fn display_tex(&self) -> (String, usize, Vec<(usize, Cursor)>) {
        let mut w = DisplayWriter {
            out: String::new(),
            positions: Vec::new(),
            caret: 0,
            cur: &self.cur,
            command: self.command.as_deref(),
        };
        w.list(&self.root, &mut Vec::new());
        if self.root.is_empty() && self.command.is_none() {
            w.out.push_str("\\placeholder");
        }
        (w.out, w.caret, w.positions)
    }

    /// Lays the editor out: the formula, the caret and the selection.
    pub fn layout(&self, font: &MathFont<'_>, opts: &RenderOptions) -> Result<EditorLayout> {
        let (tex, caret, positions) = self.display_tex();
        let mut opts = opts.clone();
        opts.hit_testing = true;
        let display = crate::render(font, &tex, &opts)?;
        let caret = caret_rect(&display, caret, opts.font_size);
        let selection = if self.has_selection() {
            let (a, b) = self.selection_range();
            let offset = |p: usize| {
                positions
                    .iter()
                    .find(|(_, c)| c.path == self.cur.path && c.pos == p)
                    .map(|(o, _)| *o as u32)
                    .unwrap_or(0)
            };
            let (start, end) = (offset(a), offset(b));
            outermost_within(&display.regions, start, end)
        } else {
            Vec::new()
        };
        Ok(EditorLayout {
            display,
            caret,
            selection,
            positions,
        })
    }

    /// Puts the cursor where a tap or click at (x, y) lands in `layout`.
    ///
    /// A tap while a `\command` is being typed finishes the command, which
    /// changes the formula the layout showed, so the cursor stays after it;
    /// tap again to move. A layout of an older state is ignored likewise.
    pub fn tap(&mut self, layout: &EditorLayout, x: f32, y: f32) {
        if self.command.is_some() {
            self.finish_command();
            return;
        }
        self.anchor = None;
        let Some(r) = layout.display.hit_innermost(x, y).or_else(|| layout.display.hit_nearest(x, y)) else {
            self.cur = Cursor {
                path: vec![],
                pos: if x <= 0.0 { 0 } else { self.root.len() },
            };
            return;
        };
        let target = if x < r.x + r.width / 2.0 { r.start } else { r.end } as usize;
        // The deepest position at, or just past, that offset.
        if let Some(c) = layout
            .positions
            .iter()
            .filter(|(o, _)| *o >= target)
            .min_by_key(|(o, c)| (*o - target, usize::MAX - c.path.len()))
            .map(|(_, c)| c.clone())
        {
            let before = std::mem::replace(&mut self.cur, c);
            if !self.cursor_is_valid() {
                self.cur = before;
            }
        }
    }
}

/// The caret at a source offset: after the atom that ends there, else
/// before the one that starts there, as tall as a line of text at its level.
fn caret_rect(display: &DisplayList, offset: usize, font_size: f32) -> Rect {
    let o = offset as u32;
    let deepest = |f: &dyn Fn(&Region) -> bool| display.regions.iter().filter(|r| f(r)).max_by_key(|r| r.depth).copied();
    let (x, r) = if let Some(r) = deepest(&|r| r.end == o && r.end > r.start) {
        (r.x + r.width, Some(r))
    } else if let Some(r) = deepest(&|r| r.start == o) {
        (r.x, Some(r))
    } else {
        (0.0, None)
    };
    match r {
        Some(r) => {
            // Never shorter than most of an em, centred on what it touches,
            // so a caret beside "-" is not a dash itself.
            let min = 0.5 * font_size;
            let h = r.height.max(min);
            let cy = r.y + r.height / 2.0;
            Rect {
                x,
                y: cy - h / 2.0,
                width: (font_size / 24.0).max(1.0),
                height: h,
            }
        }
        None => Rect {
            x,
            y: 0.0,
            width: (font_size / 24.0).max(1.0),
            height: display.height().max(font_size),
        },
    }
}

/// The outermost regions inside [start, end): one per selected atom.
fn outermost_within(regions: &[Region], start: u32, end: u32) -> Vec<Rect> {
    let inside: Vec<&Region> = regions
        .iter()
        .filter(|r| r.start >= start && r.end <= end && r.end > r.start)
        .collect();
    inside
        .iter()
        .filter(|r| {
            !inside
                .iter()
                .any(|o| (o.start, o.end) != (r.start, r.end) && o.start <= r.start && o.end >= r.end)
        })
        .map(|r| Rect {
            x: r.x,
            y: r.y,
            width: r.width,
            height: r.height,
        })
        .collect()
}

/// Atoms read aloud through the ordinary speech writer.
fn speak(atoms: &[Atom], opts: &crate::a11y::SpeechOptions) -> String {
    let mut tex = String::new();
    write_list(atoms, &mut tex);
    crate::parse(&tex).map(|n| crate::a11y::speech_with(&n, opts)).unwrap_or(tex)
}

fn list_at<'a>(root: &'a Vec<Atom>, path: &[(usize, usize)]) -> &'a Vec<Atom> {
    path.iter().fold(root, |l, &(i, k)| l[i].slot(k))
}

fn list_at_mut<'a>(root: &'a mut Vec<Atom>, path: &[(usize, usize)]) -> &'a mut Vec<Atom> {
    path.iter().fold(root, |l, &(i, k)| l[i].slot_mut(k))
}

/// Writes TeX for a list. A command name followed by a letter gets a space.
fn write_list(list: &[Atom], out: &mut String) {
    for (i, a) in list.iter().enumerate() {
        separate(out, a);
        write_atom(a, out, needs_base(list, i));
    }
}

/// Scripts with nothing to attach to (first in a list, or right after other
/// scripts, as deleting a base leaves them) get an empty base: `{}^2`.
fn needs_base(list: &[Atom], i: usize) -> bool {
    matches!(list[i], Atom::Scripts { .. }) && (i == 0 || matches!(list[i - 1], Atom::Scripts { .. }))
}

/// `\alpha` before `x` needs a space; before `+` it does not.
fn separate(out: &mut String, next: &Atom) {
    let ends_in_command = {
        let tail: String = out.chars().rev().take_while(|c| c.is_ascii_alphabetic()).collect();
        !tail.is_empty() && out[..out.len() - tail.len()].ends_with('\\')
    };
    let starts_with_letter = match next {
        Atom::Sym(s) => s.starts_with(|c: char| c.is_alphanumeric()),
        _ => false,
    };
    if ends_in_command && starts_with_letter {
        out.push(' ');
    }
}

fn write_atom(a: &Atom, out: &mut String, bare: bool) {
    let slot = |l: &[Atom], out: &mut String| {
        out.push('{');
        write_list(l, out);
        out.push('}');
    };
    match a {
        Atom::Sym(s) => out.push_str(s),
        Atom::Frac(n, d) => {
            out.push_str("\\frac");
            slot(n, out);
            slot(d, out);
        }
        Atom::Sqrt(r) => {
            out.push_str("\\sqrt");
            slot(r, out);
        }
        Atom::Root(i, r) => {
            out.push_str("\\sqrt[");
            write_list(i, out);
            out.push(']');
            slot(r, out);
        }
        Atom::Scripts { sub, sup } => {
            if bare {
                out.push_str("{}");
            }
            if let Some(s) = sub {
                out.push('_');
                slot(s, out);
            }
            if let Some(s) = sup {
                out.push('^');
                slot(s, out);
            }
        }
        Atom::Group { open, close, body } => {
            out.push_str("\\left");
            out.push_str(open);
            out.push(' ');
            write_list(body, out);
            out.push_str(" \\right");
            out.push_str(close);
        }
    }
}

/// Writes the display TeX, recording where each cursor position falls.
struct DisplayWriter<'a> {
    out: String,
    positions: Vec<(usize, Cursor)>,
    caret: usize,
    cur: &'a Cursor,
    command: Option<&'a str>,
}

impl DisplayWriter<'_> {
    fn mark(&mut self, path: &[(usize, usize)], pos: usize) {
        let here = path == self.cur.path.as_slice() && pos == self.cur.pos;
        if here {
            if let Some(cmd) = self.command {
                // The command being typed, grey, before the caret.
                self.out.push_str(&format!("{{\\color{{gray}}\\backslash\\mathrm{{{cmd}}}}}"));
            }
            self.caret = self.out.len();
        }
        self.positions.push((self.out.len(), Cursor { path: path.to_vec(), pos }));
    }

    fn list(&mut self, list: &[Atom], path: &mut Vec<(usize, usize)>) {
        for (i, a) in list.iter().enumerate() {
            separate(&mut self.out, a);
            self.mark(path, i);
            self.atom(a, i, needs_base(list, i), path);
        }
        self.mark(path, list.len());
    }

    fn slot(&mut self, list: &[Atom], path: &mut Vec<(usize, usize)>, i: usize, k: usize, open: char, close: char) {
        self.out.push(open);
        path.push((i, k));
        let empty_here = list.is_empty() && !(path.as_slice() == self.cur.path.as_slice() && self.command.is_some());
        self.list(list, path);
        if empty_here {
            self.out.push_str("\\placeholder");
        }
        path.pop();
        self.out.push(close);
    }

    fn atom(&mut self, a: &Atom, i: usize, bare: bool, path: &mut Vec<(usize, usize)>) {
        match a {
            Atom::Sym(s) => self.out.push_str(s),
            Atom::Frac(n, d) => {
                self.out.push_str("\\frac");
                self.slot(n, path, i, 0, '{', '}');
                self.slot(d, path, i, 1, '{', '}');
            }
            Atom::Sqrt(r) => {
                self.out.push_str("\\sqrt");
                self.slot(r, path, i, 0, '{', '}');
            }
            Atom::Root(idx, r) => {
                self.out.push_str("\\sqrt");
                self.slot(idx, path, i, 0, '[', ']');
                self.slot(r, path, i, 1, '{', '}');
            }
            Atom::Scripts { sub, sup } => {
                if bare {
                    self.out.push_str("{}");
                }
                if let Some(s) = sub {
                    self.out.push('_');
                    self.slot(s, path, i, 0, '{', '}');
                }
                if let Some(s) = sup {
                    self.out.push('^');
                    self.slot(s, path, i, 1, '{', '}');
                }
            }
            Atom::Group { open, close, body } => {
                self.out.push_str("\\left");
                self.out.push_str(open);
                self.out.push('{');
                path.push((i, 0));
                let empty = body.is_empty();
                self.list(body, path);
                if empty {
                    self.out.push_str("\\placeholder");
                }
                path.pop();
                self.out.push_str("}\\right");
                self.out.push_str(close);
            }
        }
    }
}

/// Reads TeX into editor atoms: the structures the editor edits, and
/// everything else as opaque atoms that keep their source.
struct TexReader<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> TexReader<'a> {
    fn new(s: &'a str) -> Self {
        TexReader { s, i: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.i += c.len_utf8();
        Some(c)
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }

    /// A list, up to `stop` (a closing `}` or `]`, or `\right` when `stop`
    /// is `\0`) or the end.
    fn list(&mut self, stop: Option<char>) -> Vec<Atom> {
        let mut out = vec![];
        // After an empty group, `^` starts new scripts rather than adding to
        // the ones before: `x^a{}_b` is two script atoms.
        let mut barrier = false;
        loop {
            self.skip_ws();
            let Some(c) = self.peek() else { break };
            if stop == Some('\0') && self.s[self.i..].starts_with("\\right") {
                self.i += "\\right".len();
                break;
            }
            if Some(c) == stop {
                self.bump();
                break;
            }
            match c {
                '{' => {
                    self.bump();
                    let inner = self.list(Some('}'));
                    barrier = inner.is_empty();
                    out.extend(inner);
                    continue;
                }
                // An unmatched `}` is not a symbol; a lone `]` is one.
                '}' => {
                    self.bump();
                }
                '^' | '_' => {
                    self.bump();
                    let body = self.arg();
                    let merge = !barrier
                        && matches!(out.last(), Some(Atom::Scripts { sub, sup }) if (c == '^' && sup.is_none()) || (c == '_' && sub.is_none()));
                    if merge {
                        if let Some(Atom::Scripts { sub, sup }) = out.last_mut() {
                            if c == '^' {
                                *sup = Some(body);
                            } else {
                                *sub = Some(body);
                            }
                        }
                    } else if c == '^' {
                        out.push(Atom::Scripts {
                            sub: None,
                            sup: Some(body),
                        });
                    } else {
                        out.push(Atom::Scripts {
                            sub: Some(body),
                            sup: None,
                        });
                    }
                }
                '\\' => match self.command() {
                    Atom::Sym(s) if s.is_empty() => {}
                    a => out.push(a),
                },
                _ => {
                    self.bump();
                    out.push(Atom::Sym(c.to_string()));
                }
            }
            barrier = false;
        }
        out
    }

    /// One argument: a braced group or a single token.
    fn arg(&mut self) -> Vec<Atom> {
        self.skip_ws();
        match self.peek() {
            Some('{') => {
                self.bump();
                self.list(Some('}'))
            }
            Some('\\') => vec![self.command()],
            Some(_) => {
                let c = self.bump().unwrap();
                vec![Atom::Sym(c.to_string())]
            }
            None => vec![],
        }
    }

    fn name(&mut self) -> String {
        let start = self.i;
        self.bump(); // the backslash
        match self.peek() {
            Some(c) if c.is_ascii_alphabetic() => {
                while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                    self.bump();
                }
            }
            Some(_) => {
                self.bump();
            }
            None => {}
        }
        self.s[start..self.i].to_string()
    }

    fn command(&mut self) -> Atom {
        let start = self.i;
        let name = self.name();
        match name.as_str() {
            "\\frac" | "\\dfrac" | "\\tfrac" => {
                let n = self.arg();
                let d = self.arg();
                Atom::Frac(n, d)
            }
            "\\sqrt" => {
                self.skip_ws();
                if self.peek() == Some('[') {
                    self.bump();
                    let idx = self.list(Some(']'));
                    let r = self.arg();
                    Atom::Root(idx, r)
                } else {
                    Atom::Sqrt(self.arg())
                }
            }
            "\\left" => {
                let open = self.delimiter();
                let body = self.until_right();
                let close = self.delimiter();
                Atom::Group { open, close, body }
            }
            "\\placeholder" => Atom::Sym(String::new()),
            "\\begin" => {
                // An environment is kept whole: up to its matching \end.
                let env = self.arg_raw();
                let end = format!("\\end{{{env}}}");
                match self.s[self.i..].find(&end) {
                    Some(k) => self.i += k + end.len(),
                    None => self.i = self.s.len(),
                }
                Atom::Sym(self.s[start..self.i].to_string())
            }
            n if n.len() > 1 && takes_argument(&n[1..]) => {
                // \mathbf{x}, \text{...}, \hat{x}: kept whole, with arguments.
                let mut count = arguments(&n[1..]);
                while count > 0 {
                    self.skip_ws();
                    if self.peek() == Some('{') {
                        self.arg_raw();
                    } else {
                        self.arg();
                    }
                    count -= 1;
                }
                Atom::Sym(self.s[start..self.i].to_string())
            }
            _ => Atom::Sym(name),
        }
    }

    fn arg_raw(&mut self) -> String {
        self.skip_ws();
        if self.peek() != Some('{') {
            return String::new();
        }
        let start = self.i + 1;
        let mut depth = 0;
        while let Some(c) = self.bump() {
            match c {
                '\\' => {
                    self.bump();
                }
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return self.s[start..self.i - 1].to_string();
                    }
                }
                _ => {}
            }
        }
        self.s[start..].to_string()
    }

    fn delimiter(&mut self) -> String {
        self.skip_ws();
        match self.peek() {
            Some('\\') => self.name()[..].to_string(),
            Some(_) => self.bump().unwrap().to_string(),
            None => ".".into(),
        }
    }

    /// The body of `\left ... \right`, which nests.
    fn until_right(&mut self) -> Vec<Atom> {
        self.list(Some('\0'))
    }
}

/// Commands whose arguments belong to them, kept as one opaque atom.
fn takes_argument(name: &str) -> bool {
    arguments(name) > 0
}

fn arguments(name: &str) -> usize {
    match name {
        "text" | "mathrm" | "mathbf" | "mathit" | "mathbb" | "mathcal" | "mathfrak" | "mathsf" | "mathtt" | "boldsymbol"
        | "operatorname" | "hat" | "bar" | "vec" | "dot" | "ddot" | "tilde" | "widehat" | "widetilde" | "overline" | "underline"
        | "overbrace" | "underbrace" | "boxed" | "cancel" | "ce" | "pu" | "mbox" => 1,
        "binom" | "color" | "textcolor" | "overset" | "underset" | "stackrel" => 2,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(s: &str) -> Editor {
        let mut e = Editor::new();
        e.type_text(s);
        e
    }

    #[test]
    fn typing_builds_structure() {
        assert_eq!(typed("x^2+1").tex(), "x^{2+1}");
        let mut e = typed("x^2");
        e.key(Key::Right);
        e.type_text("+1");
        assert_eq!(e.tex(), "x^{2}+1");
        assert_eq!(typed("1/2").tex(), r"\frac{1}{2}");
        assert_eq!(typed("a+bc/d").tex(), r"a+\frac{bc}{d}");
        assert_eq!(typed("/").tex(), r"\frac{}{}");
        assert_eq!(typed("x_1").tex(), "x_{1}");
        assert_eq!(typed("(a+b").tex(), r"\left( a+b \right)");
        assert_eq!(typed("(a+b)/2").tex(), r"\frac{\left( a+b \right)}{2}");
        assert_eq!(typed("sqrt2").tex(), r"\sqrt{2}");
        assert_eq!(typed("2pi r").tex(), r"2\pi r");
        assert_eq!(typed("sinx").tex(), r"\sin x");
        assert_eq!(typed(r"\alpha +1").tex(), r"\alpha+1");
        assert_eq!(typed(r"\frac 1").tex(), r"\frac{1}{}");
        assert_eq!(typed("|x|+1").tex(), r"\left| x \right|+1");
    }

    #[test]
    fn arrows_walk_the_structure() {
        let mut e = typed("1/2");
        e.key(Key::Up);
        e.type_text("0");
        assert_eq!(e.tex(), r"\frac{10}{2}");
        e.key(Key::Down);
        e.key(Key::End);
        e.type_text("3");
        assert_eq!(e.tex(), r"\frac{10}{23}");
        e.key(Key::Right);
        e.type_text("+x");
        assert_eq!(e.tex(), r"\frac{10}{23}+x");
        // Left from the end walks back into the denominator.
        let mut e = typed("1/2");
        e.key(Key::Right);
        e.key(Key::Left);
        e.type_text("5");
        assert_eq!(e.tex(), r"\frac{1}{25}");
    }

    #[test]
    fn backspace_takes_structures_apart() {
        let mut e = typed("1/2");
        e.key(Key::Backspace);
        e.key(Key::Backspace);
        // Start of the denominator: on to the end of the numerator.
        e.key(Key::Backspace);
        assert_eq!(e.tex(), r"\frac{}{}");
        e.key(Key::Backspace);
        assert_eq!(e.tex(), "");
        let mut e = typed("x^2");
        e.key(Key::Left);
        e.key(Key::Backspace);
        assert_eq!(e.tex(), "x2");
    }

    #[test]
    fn selection_wraps_and_deletes() {
        let mut e = typed("a+b");
        e.key(Key::SelectAll);
        e.type_text("(");
        assert_eq!(e.tex(), r"\left( a+b \right)");
        let mut e = typed("a+b");
        e.key(Key::SelectLeft);
        e.type_text("/");
        assert_eq!(e.tex(), r"a+\frac{b}{}");
        let mut e = typed("a+b");
        e.key(Key::SelectAll);
        e.key(Key::Backspace);
        assert!(e.is_empty());
    }

    #[test]
    fn the_cursor_is_described() {
        use crate::a11y::{SpeechOptions, Verbosity};
        use crate::speech_lang::Language;
        let en = SpeechOptions {
            verbosity: Verbosity::Brief,
            language: Language::English,
        };
        let mut e = typed("1/");
        assert_eq!(e.describe(&en), "denominator, blank");
        e.type_text("2");
        assert_eq!(e.describe(&en), "denominator, 2");
        let mut e = typed("x^2");
        assert_eq!(e.describe(&en), "superscript, 2");
        e.key(Key::Right);
        assert_eq!(e.describe(&en), "x squared");
        let es = SpeechOptions {
            verbosity: Verbosity::Brief,
            language: Language::Spanish,
        };
        assert!(
            typed("1/").describe(&es).starts_with("denominador"),
            "{}",
            typed("1/").describe(&es)
        );
        assert_eq!(Key::from_name("ArrowLeft", true, false), Some(Key::SelectLeft));
        assert_eq!(Key::from_name("z", true, true), Some(Key::Redo));
        assert_eq!(Key::from_name("x", false, false), None);
    }

    #[test]
    fn pasted_tex_keeps_its_structure() {
        let mut e = typed("x+");
        e.insert_tex(r"\frac{a}{b}");
        e.type_text("+1");
        assert_eq!(e.tex(), r"x+\frac{a}{b}+1");
    }

    #[test]
    fn undo_and_redo() {
        let mut e = typed("ab");
        e.key(Key::Undo);
        assert_eq!(e.tex(), "a");
        e.key(Key::Redo);
        assert_eq!(e.tex(), "ab");
    }

    #[test]
    fn tex_round_trips() {
        for tex in [
            r"\frac{1}{2}",
            r"x^{2}+y_{1}^{3}",
            r"\sqrt[3]{x}",
            r"\left( a+b \right)",
            r"\int_{0}^{1}x\,dx",
            r"\begin{pmatrix}a&b\\c&d\end{pmatrix}+\mathbf{v}",
            r"\text{if } x>0",
        ] {
            let e = Editor::from_tex(tex);
            let again = Editor::from_tex(&e.tex());
            assert_eq!(e.tex(), again.tex(), "{tex}");
            crate::parse(&e.tex()).unwrap_or_else(|err| panic!("{tex} -> {}: {err}", e.tex()));
        }
    }

    #[test]
    fn caret_and_tap_follow_the_drawing() {
        let font = crate::bundled::font().unwrap();
        let opts = RenderOptions {
            font_size: 40.0,
            ..RenderOptions::default()
        };
        let mut e = typed("x+1/2");
        let l = e.layout(&font, &opts).unwrap();
        // Caret in the denominator: right of the 2, below the axis.
        assert!(l.caret.y > l.display.ascent * 0.5, "{:?}", l.caret);
        // Tap left of everything: start of the formula.
        e.tap(&l, 1.0, l.display.ascent);
        e.type_text("y");
        assert_eq!(e.tex(), r"yx+\frac{1}{2}");
        // An empty editor draws a placeholder and a caret.
        let l = Editor::new().layout(&font, &opts).unwrap();
        assert!(l.display.items.len() >= 4 && l.caret.height > 0.0);
    }

    #[test]
    fn random_sessions_never_break() {
        // Thousands of random keystrokes: no panic, every state draws, the
        // TeX always parses, and reading it back gives the same TeX.
        let font = crate::bundled::font().unwrap();
        let opts = RenderOptions::default();
        let keys = [
            Key::Left,
            Key::Right,
            Key::Up,
            Key::Down,
            Key::Home,
            Key::End,
            Key::Backspace,
            Key::Delete,
            Key::SelectLeft,
            Key::SelectRight,
            Key::SelectAll,
            Key::Enter,
            Key::Undo,
            Key::Redo,
        ];
        let text = [
            "x", "2", "+", "=", "/", "^", "_", "(", ")", "[", "]", "|", "\\", "a", "s", "q", "r", "t", " ", "pi", "{", "}", "*", "<",
        ];
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = move |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        for _session in 0..40 {
            let mut e = Editor::new();
            let mut l = e.layout(&font, &opts).unwrap();
            let mut log: Vec<String> = vec![];
            for _ in 0..250 {
                let before = e.clone();
                match next(4) {
                    0 => {
                        let k = keys[next(keys.len())];
                        log.push(format!("{k:?}"));
                        e.key(k)
                    }
                    1 => {
                        let (x, y) = (next(400) as f32, next(60) as f32);
                        log.push(format!("tap {x},{y}"));
                        e.tap(&l, x, y)
                    }
                    _ => {
                        let t = text[next(text.len())];
                        log.push(format!("{t:?}"));
                        e.type_text(t)
                    }
                }
                assert!(
                    e.cursor_is_valid(),
                    "cursor {:?} invalid after {:?}\nbefore: {:?} cur {:?} anchor {:?} cmd {:?}",
                    e.cur,
                    log.iter().rev().take(6).collect::<Vec<_>>(),
                    before.root,
                    before.cur,
                    before.anchor,
                    before.command
                );
                l = e.layout(&font, &opts).unwrap_or_else(|err| panic!("{}: {err}", e.display_tex().0));
                let tex = e.tex();
                crate::parse(&tex).unwrap_or_else(|err| panic!("{tex}: {err}"));
                assert_eq!(Editor::from_tex(&tex).tex(), tex);
                assert!(l.caret.height > 0.0 && l.caret.x.is_finite());
            }
        }
    }

    #[test]
    fn every_state_renders() {
        // Every prefix of a long typing session must lay out.
        let font = crate::bundled::font().unwrap();
        let opts = RenderOptions::default();
        let script = r"x=(-b+-sqrt(b^2-4ac))/(2a) \alpha sum_i=1^n int_0^1 |x| 1/";
        let mut e = Editor::new();
        for c in script.chars() {
            e.type_text(&c.to_string());
            e.layout(&font, &opts)
                .unwrap_or_else(|err| panic!("after {c:?}: {} {err}", e.display_tex().0));
            assert!(crate::parse(&e.tex()).is_ok(), "{}", e.tex());
        }
        for k in [
            Key::Left,
            Key::Left,
            Key::Up,
            Key::Backspace,
            Key::SelectLeft,
            Key::SelectLeft,
            Key::Delete,
            Key::Down,
            Key::Undo,
        ] {
            e.key(k);
            e.layout(&font, &opts).unwrap_or_else(|err| panic!("after {k:?}: {err}"));
        }
    }
}
