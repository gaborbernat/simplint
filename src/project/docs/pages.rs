use super::{DocReferences, Module};

const ROLES: [&str; 12] =
    ["func", "class", "meth", "attr", "data", "obj", "mod", "exc", "const", "type", "deco", "property"];

const OBJECTS: [&str; 10] = [
    "function",
    "class",
    "method",
    "attribute",
    "data",
    "exception",
    "decorator",
    "property",
    "classmethod",
    "staticmethod",
];

impl DocReferences {
    pub(super) fn read(&mut self, text: &str) {
        let mut open: Option<Directive> = None;
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some((name, argument)) = directive(trimmed) {
                if let Some(done) = open.replace(Directive { name, argument, options: Vec::new() }) {
                    done.record(self);
                }
            } else if let (Some(directive), Some(option)) = (open.as_mut(), option(trimmed)) {
                directive.options.push(option);
            } else if let Some(done) = open.take() {
                done.record(self);
            }
            self.names.extend(role_targets(line));
        }
        if let Some(done) = open {
            done.record(self);
        }
    }
}

// A directive and the options read under it so far.
struct Directive<'t> {
    name: &'t str,
    argument: &'t str,
    options: Vec<(&'t str, &'t str)>,
}

impl Directive<'_> {
    // The names a directive documents. A name relative to the page's `currentmodule` needs no resolving, since names
    // match by their dotted suffix.
    fn record(self, references: &mut DocReferences) {
        let value = |key: &str| self.options.iter().find(|(option, _)| *option == key).map(|(_, value)| *value);
        let target = self.argument.split(['(', ' ']).next().unwrap_or_default().to_owned();
        let name = self.name.strip_prefix("py:").unwrap_or(self.name);
        let object = name.strip_prefix("auto").unwrap_or(name);
        if matches!(object, "module" | "class" | "exception") && name.starts_with("auto") {
            // `:members:` alone documents every member, and with a list only those it names.
            let exclude = listed(value("exclude-members").unwrap_or_default()).map(str::to_owned).collect();
            match value("members") {
                Some("") if object == "module" => {
                    references.modules.insert(Module {
                        target: target.clone(),
                        exclude,
                        private: value("private-members").is_some(),
                        special: value("special-members").is_some(),
                    });
                }
                Some("") => _ = references.scopes.insert(format!("{target}.")),
                Some(members) => {
                    references.names.extend(listed(members).map(|member| format!("{target}.{member}")));
                }
                None => {}
            }
            references.names.insert(target);
        } else if OBJECTS.contains(&object) {
            references.names.insert(target);
        }
        // sphinx-argparse and sphinx-argparse-cli render the parser a module's function builds.
        if let Some(module) = value("module")
            && let Some(function) = value("func")
        {
            references.names.insert(format!("{module}.{function}"));
        }
        if let Some(path) = value("ref") {
            references.names.insert(path.to_owned());
        }
    }
}

fn listed(value: &str) -> impl Iterator<Item = &str> {
    value.split(',').map(str::trim).filter(|member| !member.is_empty())
}

// `.. name:: argument` in reStructuredText, or ```` ```{name} argument ```` and `:::{name} argument` in MyST.
fn directive(line: &str) -> Option<(&str, &str)> {
    if let Some(rest) = line.strip_prefix(".. ") {
        let (name, argument) = rest.split_once("::")?;
        return (!name.contains(' ')).then(|| (name, argument.trim()));
    }
    let rest = line.trim_start_matches(['`', ':']).strip_prefix('{').filter(|_| line.starts_with(['`', ':']))?;
    let (name, argument) = rest.split_once('}')?;
    Some((name, argument.trim()))
}

// `:key: value`, the option lines under a directive.
fn option(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.strip_prefix(':')?.split_once(':')?;
    (!key.is_empty() && !key.contains(' ')).then(|| (key, value.trim()))
}

// The targets of the line's Python roles: `:func:`name``, `:py:meth:`~pkg.Class.name``, `:class:`Title <pkg.Name>``,
// and MyST's `{py:func}`name``. A leading `.` searches every module, which the relative match already does.
fn role_targets(line: &str) -> Vec<String> {
    let parts: Vec<&str> = line.split('`').collect();
    (1..parts.len())
        .step_by(2)
        .filter_map(|at| {
            let (before, content) = (parts[at - 1], parts[at]);
            let role = match before.strip_suffix(':') {
                Some(rst) => rst.rsplit(':').next().unwrap_or_default(),
                None => before.strip_suffix('}')?.rsplit_once('{')?.1,
            };
            ROLES.contains(&role.strip_prefix("py:").unwrap_or(role)).then_some(())?;
            let target = content.rsplit_once('<').map_or(content, |(_, inner)| inner.trim_end_matches('>'));
            let target = target.trim_start_matches(['~', '!', '.']).trim_end_matches("()");
            (!target.is_empty()).then(|| target.to_owned())
        })
        .collect()
}
