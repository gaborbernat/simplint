// What a scan finds in a decoded pattern: the unnamed groups as the offsets of their `(`, whether the pattern refers
// to a group by number or name, which a group that does not capture would break, and the constructs a reader follows.
pub(crate) struct Scan {
    pub(crate) groups: Vec<usize>,
    pub(crate) refers: bool,
    pub(crate) named: bool,
    // Each group, alternation, character class and repeat of a group. A class with its repeat is one, and a flag or a
    // comment is none.
    pub(crate) constructs: usize,
    // The comments that explain the pattern: a `#` comment outside a character class in verbose mode, or a `(?#...)`.
    pub(crate) comments: usize,
    // Whether verbose mode is on, by the flag argument or an inline `(?x)`.
    pub(crate) verbose: bool,
}

// Walks the pattern the way `re` parses it, so a `(` in a character class, an escape, or a verbose comment does not
// count. `None` for a pattern `re` rejects, or whose verbose mode the flags leave unknown, which is left alone.
pub(crate) fn scan(pattern: &[u8], verbose: Option<bool>) -> Option<Scan> {
    let verbose = has_verbose_flag(pattern) || verbose?;
    let mut scan = Scan { groups: Vec::new(), refers: false, named: false, constructs: 0, comments: 0, verbose };
    // A scoped `(?x:...)` changes verbose mode for its own group only, so a `)` restores the mode around that group.
    let mut outer: Vec<bool> = Vec::new();
    let mut inside = verbose;
    let mut index = 0;
    while index < pattern.len() {
        let rest = &pattern[index..];
        index += match rest {
            [b'\\'] => return None,
            [b'\\', escaped, ..] => {
                scan.refers |= (b'1'..=b'9').contains(escaped);
                2
            }
            // A class and the repeat after it are one construct, as a reader takes in `[ \t]*` at once.
            [b'[', ..] => {
                scan.constructs += 1;
                class_length(rest)?
            }
            [b'#', ..] if inside => {
                scan.comments += 1;
                rest.iter().position(|&byte| byte == b'\n').unwrap_or(rest.len())
            }
            [b'(', b'?', b'#', ..] => {
                scan.comments += 1;
                rest.iter().position(|&byte| byte == b')')? + 1
            }
            // The condition and the name of a reference are not groups, and their `)` is not a group's end.
            [b'(', b'?', b'(', ..] => {
                scan.refers = true;
                scan.constructs += 1;
                outer.push(inside);
                rest.iter().position(|&byte| byte == b')')? + 1
            }
            [b'(', b'?', b'P', b'=', ..] => {
                scan.refers = true;
                rest.iter().position(|&byte| byte == b')')? + 1
            }
            [b'(', b'?', b'P', b'<', ..] | [b'(', b'?', b'<', b'a'..=b'z' | b'A'..=b'Z' | b'_', ..] => {
                scan.named = true;
                scan.constructs += 1;
                outer.push(inside);
                2
            }
            // `(?i)` sets flags for the whole pattern, which is not a group; `(?i:...)` and `(?:...)` are.
            [b'(', b'?', ..] => {
                let flags = rest[2..].iter().take_while(|byte| b"aiLmsux-".contains(byte)).count();
                if rest.get(2 + flags) == Some(&b')') {
                    3 + flags
                } else {
                    scan.constructs += 1;
                    outer.push(inside);
                    inside = scoped_verbose(&rest[2..2 + flags], inside);
                    2
                }
            }
            [b'(', ..] => {
                scan.groups.push(index);
                scan.constructs += 1;
                outer.push(inside);
                1
            }
            [b')', ..] => {
                scan.constructs += usize::from(repeats(&rest[1..]));
                inside = outer.pop().unwrap_or(inside);
                1
            }
            [b'|', ..] => {
                scan.constructs += 1;
                1
            }
            _ => 1,
        };
    }
    Some(scan)
}

// Whether verbose mode is on inside a group whose flags are `on-off`, as `(?x:` turns it on and `(?-x:` turns it off.
fn scoped_verbose(flags: &[u8], verbose: bool) -> bool {
    let (on, off) = match flags.iter().position(|&byte| byte == b'-') {
        Some(dash) => (&flags[..dash], &flags[dash + 1..]),
        None => (flags, &flags[flags.len()..]),
    };
    if off.contains(&b'x') {
        false
    } else if on.contains(&b'x') {
        true
    } else {
        verbose
    }
}

// The length of the character class at the start of `class`. A `]` right after its opening `[` or `[^` is a member.
fn class_length(class: &[u8]) -> Option<usize> {
    let mut index = 1;
    if class.get(index) == Some(&b'^') {
        index += 1;
    }
    if class.get(index) == Some(&b']') {
        index += 1;
    }
    loop {
        match class.get(index)? {
            b'\\' => index += 2,
            b']' => return Some(index + 1),
            _ => index += 1,
        }
    }
}

// Whether a repeat operator starts `rest`, which applies to the group or class before it: `*`, `+`, `?`, or a `{m,n}`
// bound with at least one digit.
fn repeats(rest: &[u8]) -> bool {
    match rest {
        [b'*' | b'+' | b'?', ..] => true,
        [b'{', ..] => rest.iter().position(|&byte| byte == b'}').is_some_and(|end| {
            let bounds = &rest[1..end];
            bounds.iter().any(u8::is_ascii_digit) && bounds.iter().all(|byte| byte.is_ascii_digit() || *byte == b',')
        }),
        _ => false,
    }
}

// An inline `(?x)` at the start of a pattern turns verbose mode on, as `re.VERBOSE` does.
fn has_verbose_flag(pattern: &[u8]) -> bool {
    pattern
        .strip_prefix(b"(?")
        .and_then(|rest| rest.iter().position(|&byte| byte == b')').map(|end| &rest[..end]))
        .is_some_and(|flags| flags.contains(&b'x') && flags.iter().all(|flag| b"aiLmsux-".contains(flag)))
}
