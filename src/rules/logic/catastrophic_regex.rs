use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::regex_pattern::{INLINE_FLAGS, bounds, class_length, scan};

pub fn catastrophic_regex(checker: &Checker) {
    for (literal, call) in checker.state().regex_facts(checker).sites() {
        let pattern = literal.decoded();
        let Some(scan) = scan(&pattern, call.verbose) else {
            continue;
        };
        let Some(nodes) = Parser::new(&pattern, scan.verbose).alternatives() else {
            continue;
        };
        let Some(repeat) = nodes.iter().find_map(find_nested) else {
            continue;
        };
        let shown = shown(&pattern[repeat.0..repeat.1], scan.verbose);
        checker.report(
            Rule::CatastrophicRegex,
            literal.range,
            format!(
                "`{shown}` repeats a group that a repeat inside it can split in many ways, so a string that almost \
                 matches takes exponential time; make the parts inside the group exclusive, or use an atomic group or a \
                 possessive quantifier"
            ),
        );
    }
}

struct Parser<'p> {
    pattern: &'p [u8],
    index: usize,
    verbose: bool,
}

impl<'p> Parser<'p> {
    fn new(pattern: &'p [u8], verbose: bool) -> Self {
        Self { pattern, index: 0, verbose }
    }

    // The whole pattern, as the sequences of its top-level alternatives. `None` for a construct the check does not
    // read, as a conditional `(?(1)...)` or a scoped verbose flag is, which leaves the pattern unreported.
    fn alternatives(&mut self) -> Option<Vec<Node>> {
        let group = self.group_body()?;
        (self.index == self.pattern.len()).then(|| vec![Node::Group { alternatives: group, atomic: false }])
    }

    fn group_body(&mut self) -> Option<Vec<Vec<Node>>> {
        let mut alternatives = vec![Vec::new()];
        loop {
            self.skip_verbose();
            match self.pattern.get(self.index) {
                None | Some(b')') => return Some(alternatives),
                Some(b'|') => {
                    self.index += 1;
                    alternatives.push(Vec::new());
                }
                Some(_) => {
                    let start = self.index;
                    let node = self.item()?;
                    alternatives.last_mut()?.push(self.quantified(node, start)?);
                }
            }
        }
    }

    fn skip_verbose(&mut self) {
        while self.verbose {
            match self.pattern.get(self.index) {
                Some(byte) if byte.is_ascii_whitespace() => self.index += 1,
                Some(b'#') => {
                    while self.pattern.get(self.index).is_some_and(|byte| *byte != b'\n') {
                        self.index += 1;
                    }
                }
                _ => return,
            }
        }
    }

    fn item(&mut self) -> Option<Node> {
        let rest = &self.pattern[self.index..];
        match rest {
            [b'\\', ..] => {
                self.index += 2;
                Some(Node::Atom)
            }
            [b'[', ..] => {
                self.index += class_length(rest)?;
                Some(Node::Atom)
            }
            [b'(', b'?', b'#', ..] => {
                self.index += rest.iter().position(|&byte| byte == b')')? + 1;
                Some(Node::Group { alternatives: vec![Vec::new()], atomic: true })
            }
            [b'(', b'?', b'(', ..] | [b')', ..] => None,
            [b'(', b'?', b'P', b'=', ..] => {
                self.index += rest.iter().position(|&byte| byte == b')')? + 1;
                Some(Node::Atom)
            }
            [b'(', ..] => self.group(rest),
            _ => {
                self.index += 1;
                Some(Node::Atom)
            }
        }
    }

    fn group(&mut self, rest: &[u8]) -> Option<Node> {
        let (prefix, atomic, lookaround) = match rest {
            [b'(', b'?', b'P', b'<', ..] | [b'(', b'?', b'<', b'a'..=b'z' | b'A'..=b'Z' | b'_', ..] => {
                (rest.iter().position(|&byte| byte == b'>')? + 1, false, false)
            }
            [b'(', b'?', b'>', ..] => (3, true, false),
            [b'(', b'?', b'=' | b'!', ..] => (3, true, true),
            [b'(', b'?', b'<', b'=' | b'!', ..] => (4, true, true),
            [b'(', b'?', ..] => {
                let flags = rest[2..].iter().take_while(|byte| INLINE_FLAGS.contains(byte)).count();
                match rest.get(2 + flags)? {
                    // A global flag; the scan already read a verbose one.
                    b')' => {
                        self.index += 3 + flags;
                        return Some(Node::Group { alternatives: vec![Vec::new()], atomic: true });
                    }
                    // A scoped verbose flag changes how the rest of the group reads, which this parser does not track.
                    b':' if !rest[2..2 + flags].contains(&b'x') => (3 + flags, false, false),
                    _ => return None,
                }
            }
            _ => (1, false, false),
        };
        self.index += prefix;
        let alternatives = self.group_body()?;
        if self.pattern.get(self.index) != Some(&b')') {
            return None;
        }
        self.index += 1;
        // A lookaround matches no text, so a repeat of it cannot split any.
        if lookaround {
            return Some(Node::Group { alternatives: vec![Vec::new()], atomic: true });
        }
        Some(Node::Group { alternatives, atomic })
    }

    fn quantified(&mut self, node: Node, start: usize) -> Option<Node> {
        self.skip_verbose();
        let rest = &self.pattern[self.index..];
        let (length, min, unbounded) = match rest {
            [b'*', ..] => (1, 0, true),
            [b'+', ..] => (1, 1, true),
            [b'?', ..] => (1, 0, false),
            [b'{', ..] => match bounds(rest) {
                Some(found) => found,
                None => return Some(node),
            },
            _ => return Some(node),
        };
        self.index += length;
        self.skip_verbose();
        let possessive = match self.pattern.get(self.index) {
            Some(b'?') => {
                self.index += 1;
                false
            }
            Some(b'+') => {
                self.index += 1;
                true
            }
            _ => false,
        };
        let span = (start, self.index);
        self.quantified(Node::Repeat { node: Box::new(node), min, unbounded, possessive, span }, start)
    }
}

enum Node {
    // A character, class, escape, anchor or backreference: something that matches without backtracking inside.
    Atom,
    // A group's alternatives, each a sequence of nodes. An atomic group never backtracks into itself.
    Group { alternatives: Vec<Vec<Node>>, atomic: bool },
    Repeat { node: Box<Node>, min: u32, unbounded: bool, possessive: bool, span: (usize, usize) },
}

impl Node {
    fn is_optional(&self) -> bool {
        match self {
            Node::Atom => false,
            Node::Group { alternatives, .. } => {
                alternatives.iter().any(|sequence| sequence.iter().all(Node::is_optional))
            }
            Node::Repeat { node, min, .. } => *min == 0 || node.is_optional(),
        }
    }
}

// The first repeat that the input can drive into exponential backtracking: an unbounded repeat of a group where one
// alternative is a single unbounded repeat with everything else around it optional, as `(a+)+`, `(\w+\s*)*` or
// `(x|y+)+` are. A string of `y`s then splits between the outer and inner repeat in exponentially many ways.
fn find_nested(node: &Node) -> Option<(usize, usize)> {
    match node {
        Node::Atom => None,
        Node::Group { alternatives, .. } => alternatives.iter().flatten().find_map(find_nested),
        Node::Repeat { node: inner, unbounded, possessive, span, .. } => {
            if *unbounded
                && !possessive
                && let Node::Group { alternatives, atomic: false } = inner.as_ref()
                && alternatives.iter().any(|sequence| splits(sequence))
            {
                return Some(*span);
            }
            find_nested(inner)
        }
    }
}

fn splits(sequence: &[Node]) -> bool {
    sequence.iter().enumerate().any(|(index, node)| {
        let unbounded = match node {
            Node::Repeat { unbounded, possessive, node, .. } => *unbounded && !possessive && !node.is_optional(),
            Node::Group { alternatives, atomic: false } => alternatives.iter().any(|inner| splits(inner)),
            _ => false,
        };
        unbounded && sequence.iter().enumerate().all(|(other, node)| other == index || node.is_optional())
    })
}

// A verbose pattern drops its comments and the whitespace around its pieces, so the message keeps to one line.
fn shown(repeat: &[u8], verbose: bool) -> String {
    let text = String::from_utf8_lossy(repeat);
    if !verbose {
        return text.into_owned();
    }
    let mut shown = String::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        match character {
            '\\' => {
                shown.push(character);
                shown.extend(characters.next());
            }
            '#' => {
                characters.by_ref().find(|character| *character == '\n');
            }
            character if character.is_ascii_whitespace() => {}
            character => shown.push(character),
        }
    }
    shown
}
