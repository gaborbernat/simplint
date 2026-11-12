use super::{DocReferences, Module};
use crate::rst::{Directive, directives, references};

impl DocReferences {
    pub(super) fn read(&mut self, text: &str) {
        let lines: Vec<&str> = text.lines().collect();
        for directive in directives(&lines) {
            self.record(&directive);
        }
        self.names.extend(lines.iter().flat_map(|line| references(line)).filter_map(reference_name).map(str::to_owned));
    }

    // A name relative to the page's `currentmodule` needs no resolving, since names match by their dotted suffix.
    fn record(&mut self, directive: &Directive) {
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
        let value = |key: &str| directive.options.iter().find(|(option, _)| *option == key).map(|(_, value)| *value);
        let target = directive.argument.split(['(', ' ']).next().unwrap_or_default().to_owned();
        let name = directive.name.strip_prefix("py:").unwrap_or(directive.name);
        let object = name.strip_prefix("auto").unwrap_or(name);
        if matches!(object, "module" | "class" | "exception") && name.starts_with("auto") {
            // Sphinx documents every member for `:members:` alone, and only the listed ones when it has a list.
            match value("members") {
                Some("") if object == "module" => {
                    self.modules.insert(Module {
                        target: target.clone(),
                        exclude: listed(value("exclude-members").unwrap_or_default()).map(str::to_owned).collect(),
                        private: value("private-members").is_some(),
                        special: value("special-members").is_some(),
                    });
                }
                Some("") => _ = self.scopes.insert(format!("{target}.")),
                Some(members) => {
                    self.names.extend(listed(members).map(|member| format!("{target}.{member}")));
                }
                None => {}
            }
            self.names.insert(target);
        } else if OBJECTS.contains(&object) {
            self.names.insert(target);
        }
        // sphinx-argparse and sphinx-argparse-cli render the parser a module's function builds.
        if let Some(module) = value("module")
            && let Some(function) = value("func")
        {
            self.names.insert(format!("{module}.{function}"));
        }
        if let Some(path) = value("ref") {
            self.names.insert(path.to_owned());
        }
    }
}

fn listed(value: &str) -> impl Iterator<Item = &str> {
    value.split(',').map(str::trim).filter(|member| !member.is_empty())
}

// A leading `.` searches every module, which the relative match already does.
fn reference_name(target: &str) -> Option<&str> {
    let target = target.trim_start_matches(['~', '!', '.']).trim_end_matches("()");
    (!target.is_empty()).then_some(target)
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::super::DocReferences;

    #[test_case(":func:`~pkg.mod.name`", &["pkg.mod.name"] ; "tilde_prefix")]
    #[test_case(":func:`.pkg.name`", &["pkg.name"] ; "dot_prefix")]
    #[test_case(":func:`!pkg.name`", &["pkg.name"] ; "exclamation_prefix")]
    #[test_case(":func:`pkg.name()`", &["pkg.name"] ; "call_parentheses")]
    #[test_case(":class:`Title <~pkg.Thing>`", &["pkg.Thing"] ; "explicit_title")]
    #[test_case(":func:`~`", &[] ; "nothing_left_after_the_prefix")]
    fn role_targets_name_definitions(text: &str, expected: &[&str]) {
        let mut references = DocReferences::default();
        references.read(text);
        let mut names: Vec<String> = references.names.into_iter().collect();
        names.sort();
        assert_eq!(names, expected);
    }
}
