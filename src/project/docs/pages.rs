use std::path::Path;

use super::{DocReferences, Module};
use crate::rst::{Directive, myst_markup, page_lines, rst_markup};

impl DocReferences {
    pub(super) fn read(&mut self, path: &Path, text: &str) {
        let markup = if path.extension().is_some_and(|extension| extension == "md") {
            myst_markup(text)
        } else {
            rst_markup(&page_lines(text))
        };
        for directive in &markup.directives {
            self.record(directive);
        }
        self.names.extend(markup.references.iter().filter_map(|target| reference_name(target)).map(str::to_owned));
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
        let value =
            |key: &str| directive.options.iter().find(|(option, _)| option == key).map(|(_, value)| value.as_str());
        let targets: Vec<String> = directive
            .arguments
            .iter()
            .map(|signature| signature.split(['(', ' ']).next().unwrap_or_default().to_owned())
            .collect();
        let lowered = directive.name.to_ascii_lowercase();
        let name = lowered.strip_prefix("py:").unwrap_or(&lowered);
        let object = name.strip_prefix("auto").unwrap_or(name);
        if matches!(object, "module" | "class" | "exception") && name.starts_with("auto") {
            // Sphinx documents every member for `:members:` alone, and only the listed ones when it has a list.
            for target in targets {
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
            }
        } else if OBJECTS.contains(&object) {
            self.names.extend(targets);
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
    use std::path::Path;

    use test_case::test_case;

    use super::super::DocReferences;

    #[test_case(":func:`~pkg.mod.name`", &["pkg.mod.name"] ; "tilde_prefix")]
    #[test_case(":func:`.pkg.name`", &["pkg.name"] ; "dot_prefix")]
    #[test_case(":func:`!pkg.name`", &["pkg.name"] ; "exclamation_prefix")]
    #[test_case(":func:`pkg.name()`", &["pkg.name"] ; "call_parentheses")]
    #[test_case(":class:`Title <~pkg.Thing>`", &["pkg.Thing"] ; "explicit_title")]
    #[test_case(":func:`~`", &[] ; "nothing_left_after_the_prefix")]
    #[test_case("`pkg.name`:func:", &["pkg.name"] ; "suffix_role")]
    #[test_case(":func:`pkg.\nname`", &["pkg.\nname"] ; "target_across_lines_keeps_the_break")]
    #[test_case("``:func:`pkg.name` ``", &["pkg.name"] ; "role_after_an_unclosed_literal_start")]
    #[test_case("{func}`pkg.name`", &[] ; "myst_role_in_an_rst_page")]
    fn role_targets_name_definitions(text: &str, expected: &[&str]) {
        let mut references = DocReferences::default();
        references.read(Path::new("page.rst"), text);
        let mut names: Vec<String> = references.names.into_iter().collect();
        names.sort();
        assert_eq!(names, expected);
    }

    #[test_case("{func}`pkg.name`", &["pkg.name"] ; "myst_role")]
    #[test_case(":func:`pkg.name`", &[] ; "rst_role_in_a_markdown_page")]
    #[test_case("```python\n{func}`pkg.hidden`\n```\n", &[] ; "role_in_a_code_fence")]
    #[test_case("```{note}\n{func}`pkg.name`\n```\n", &["pkg.name"] ; "role_in_a_note_fence")]
    #[test_case("    {func}`pkg.hidden`\n", &[] ; "role_in_indented_code")]
    fn myst_role_targets_name_definitions(text: &str, expected: &[&str]) {
        let mut references = DocReferences::default();
        references.read(Path::new("page.md"), text);
        let mut names: Vec<String> = references.names.into_iter().collect();
        names.sort();
        assert_eq!(names, expected);
    }

    #[test_case(".. Function:: pkg.f\n", &["pkg.f"] ; "directive_name_ignores_case")]
    #[test_case(".. automodule:: pkg\n   :members: a,\n      b\n", &["pkg", "pkg.a", "pkg.b"] ; "members_continue_on_deeper_lines")]
    #[test_case(".. data:: IDLE\n          SEND_RESPONSE\n", &["IDLE", "SEND_RESPONSE"] ; "every_signature_of_a_directive_is_recorded")]
    #[test_case(".. function:: f(a,\\\n   b)\n", &["f"] ; "backslash_joins_a_signature_with_the_next_line")]
    fn directive_names(text: &str, expected: &[&str]) {
        let mut references = DocReferences::default();
        references.read(Path::new("page.rst"), text);
        let mut names: Vec<String> = references.names.into_iter().collect();
        names.sort();
        assert_eq!(names, expected);
    }
}
