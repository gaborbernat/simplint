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
        // fullmatch can fail at the end of the pattern. A compiled pattern may later be used with fullmatch.
        let open_end = !matches!(call.attr, "fullmatch" | "compile");
        let Some(repeat) = nodes.iter().find_map(|node| find_nested(node, open_end)) else {
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

    // Skip the whole pattern if any branch uses unsupported syntax.
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
                    let quantified = self.quantified(node, start);
                    alternatives.last_mut().expect("a group body holds at least one alternative").push(quantified);
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
            [b'(', b'?', b'(', ..] => None,
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
                    // The initial scan already applied any global verbose flag.
                    b')' => {
                        self.index += 3 + flags;
                        return Some(Node::Group { alternatives: vec![Vec::new()], atomic: true });
                    }
                    // A scoped x flag would require changing whitespace handling for this group.
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
        // Lookarounds consume no text for a surrounding repeat to split.
        if lookaround {
            return Some(Node::Group { alternatives: vec![Vec::new()], atomic: true });
        }
        Some(Node::Group { alternatives, atomic })
    }

    fn quantified(&mut self, node: Node, start: usize) -> Node {
        self.skip_verbose();
        let rest = &self.pattern[self.index..];
        let (length, min, unbounded) = match rest {
            [b'*', ..] => (1, 0, true),
            [b'+', ..] => (1, 1, true),
            [b'?', ..] => (1, 0, false),
            [b'{', ..] => match bounds(rest) {
                Some(found) => found,
                None => return node,
            },
            _ => return node,
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
    Atom,
    Group { alternatives: Vec<Vec<Node>>, atomic: bool },
    Repeat { node: Box<Node>, min: u32, unbounded: bool, possessive: bool, span: (usize, usize) },
}

impl Node {
    // Lookarounds share this empty atomic group with comments and global flags. A lookaround can still fail.
    fn never_fails(&self) -> bool {
        match self {
            Node::Atom => false,
            Node::Group { alternatives, atomic: true } if alternatives.iter().all(Vec::is_empty) => false,
            Node::Group { alternatives, .. } => {
                alternatives.iter().any(|sequence| sequence.iter().all(Node::never_fails))
            }
            Node::Repeat { node, min, .. } => *min == 0 || node.never_fails(),
        }
    }

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

// Report ambiguous nested repeats only when a later failure could force the engine to try another split.
fn find_nested(node: &Node, safe_tail: bool) -> Option<(usize, usize)> {
    match node {
        Node::Atom => None,
        Node::Group { alternatives, .. } => alternatives.iter().find_map(|sequence| {
            sequence.iter().enumerate().find_map(|(index, node)| {
                find_nested(node, safe_tail && sequence[index + 1..].iter().all(Node::never_fails))
            })
        }),
        Node::Repeat { node: inner, unbounded, possessive, span, .. } => {
            if !safe_tail
                && *unbounded
                && !possessive
                && let Node::Group { alternatives, atomic: false } = inner.as_ref()
                && alternatives.iter().any(|sequence| splits(sequence))
            {
                return Some(*span);
            }
            find_nested(inner, safe_tail)
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

// Keep a multiline verbose pattern on one diagnostic line.
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
