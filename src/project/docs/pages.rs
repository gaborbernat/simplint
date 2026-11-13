use std::path::Path;

use super::{AutoclassContent, Directives, DocReferences, Module};
use crate::rst::{Directive, myst_markup, page_lines, rst_markup};

impl DocReferences {
    pub(super) fn read(&mut self, path: &Path, text: &str, directives: &mut Directives) {
        let markup = if path.extension().is_some_and(|extension| extension == "md") {
            myst_markup(text)
        } else {
            rst_markup(&page_lines(text))
        };
        let mut module = None;
        for directive in &markup.directives {
            self.record(directive, &mut module, directives);
        }
        self.names.extend(markup.references.iter().filter_map(|target| reference_name(target)).map(str::to_owned));
    }

    // A name relative to the page's `currentmodule` needs no resolving, since names match by their dotted suffix.
    fn record(&mut self, directive: &Directive, module: &mut Option<String>, directives: &mut Directives) {
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
        match name {
            "module" => *module = directive.arguments.first().cloned(),
            "currentmodule" => *module = directive.arguments.first().filter(|name| name.as_str() != "None").cloned(),
            _ => {}
        }
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
                if matches!(object, "class" | "exception") {
                    directives.record(
                        &target,
                        module.as_deref(),
                        value("class-doc-from").map(AutoclassContent::from_option),
                    );
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

impl Directives {
    // Sphinx imports a dotted target as the module path it names, and resolves a simple name against the page's module.
    // Without a module, a simple name stands for any class of that name.
    fn record(&mut self, target: &str, module: Option<&str>, option: Option<AutoclassContent>) {
        let (names, name) = match module {
            Some(module) if !target.contains('.') => (&mut self.qualified, format!("{module}.{target}")),
            _ if target.contains('.') => (&mut self.qualified, target.to_owned()),
            _ => (&mut self.simple, target.to_owned()),
        };
        let found = names.entry(name).or_default();
        *found = option.or(*found);
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
    use std::collections::BTreeMap;
    use std::path::Path;

    use test_case::test_case;

    use super::super::{AutoclassContent, Directives, DocReferences};

    #[test_case("page.rst", ":func:`~pkg.mod.name`", &["pkg.mod.name"] ; "tilde_prefix")]
    #[test_case("page.rst", ":func:`.pkg.name`", &["pkg.name"] ; "dot_prefix")]
    #[test_case("page.rst", ":func:`!pkg.name`", &["pkg.name"] ; "exclamation_prefix")]
    #[test_case("page.rst", ":func:`pkg.name()`", &["pkg.name"] ; "call_parentheses")]
    #[test_case("page.rst", ":class:`Title <~pkg.Thing>`", &["pkg.Thing"] ; "explicit_title")]
    #[test_case("page.rst", ":func:`~`", &[] ; "nothing_left_after_the_prefix")]
    #[test_case("page.rst", "`pkg.name`:func:", &["pkg.name"] ; "suffix_role")]
    #[test_case("page.rst", ":func:`pkg.\nname`", &["pkg.\nname"] ; "target_across_lines_keeps_the_break")]
    #[test_case("page.rst", "``:func:`pkg.name` ``", &["pkg.name"] ; "role_after_an_unclosed_literal_start")]
    #[test_case("page.rst", "{func}`pkg.name`", &[] ; "myst_role_in_an_rst_page")]
    #[test_case("page.md", "{func}`pkg.name`", &["pkg.name"] ; "myst_role")]
    #[test_case("page.md", ":func:`pkg.name`", &[] ; "rst_role_in_a_markdown_page")]
    #[test_case("page.md", "```python\n{func}`pkg.hidden`\n```\n", &[] ; "role_in_a_code_fence")]
    #[test_case("page.md", "```{note}\n{func}`pkg.name`\n```\n", &["pkg.name"] ; "role_in_a_note_fence")]
    #[test_case("page.md", "    {func}`pkg.hidden`\n", &[] ; "role_in_indented_code")]
    #[test_case("page.rst", ".. Function:: pkg.f\n", &["pkg.f"] ; "directive_name_ignores_case")]
    #[test_case(
        "page.rst", ".. automodule:: pkg\n   :members: a,\n      b\n", &["pkg", "pkg.a", "pkg.b"] ;
        "members_continue_on_deeper_lines"
    )]
    #[test_case(
        "page.rst", ".. data:: IDLE\n          SEND_RESPONSE\n", &["IDLE", "SEND_RESPONSE"] ;
        "every_signature_of_a_directive_is_recorded"
    )]
    #[test_case("page.rst", ".. function:: f(a,\\\n   b)\n", &["f"] ; "backslash_joins_a_signature_with_the_next_line")]
    fn names_read_from_a_page(page: &str, text: &str, expected: &[&str]) {
        let mut references = DocReferences::default();
        references.read(Path::new(page), text, &mut Directives::default());
        let mut names: Vec<String> = references.names.into_iter().collect();
        names.sort();
        assert_eq!(names, expected);
    }

    #[test_case(
        ".. currentmodule:: pkg.a\n\n.. autoclass:: Reader\n    :class-doc-from: init\n",
        &[("pkg.a.Reader", Some(AutoclassContent::Init))],
        &[] ;
        "current_module_qualifies_a_name"
    )]
    #[test_case(
        ".. py:module:: pkg.a\n\n.. autoexception:: Reader\n",
        &[("pkg.a.Reader", None)],
        &[] ;
        "module_directive_and_autoexception"
    )]
    #[test_case(
        ".. currentmodule:: pkg.a\n\n.. currentmodule:: None\n\n.. autoclass:: Reader\n",
        &[],
        &[("Reader", None)] ;
        "none_resets_the_module"
    )]
    #[test_case(
        ".. currentmodule:: pkg.a\n\n.. autoclass:: pkg.b.Reader\n    :class-doc-from: both\n",
        &[("pkg.b.Reader", Some(AutoclassContent::Both))],
        &[] ;
        "dotted_name_is_a_module_path"
    )]
    #[test_case(".. autoclass:: Reader\n", &[], &[("Reader", None)] ; "no_module_names_the_simple_name")]
    fn class_directives_are_keyed_as_sphinx_resolves_them(
        text: &str,
        qualified: &[(&str, Option<AutoclassContent>)],
        simple: &[(&str, Option<AutoclassContent>)],
    ) {
        let mut directives = Directives::default();
        DocReferences::default().read(Path::new("page.rst"), text, &mut directives);
        let owned = |pairs: &[(&str, Option<AutoclassContent>)]| -> BTreeMap<String, Option<AutoclassContent>> {
            pairs.iter().map(|(name, option)| ((*name).to_owned(), *option)).collect()
        };
        let (found_qualified, found_simple) = (
            directives.qualified.into_iter().collect::<BTreeMap<_, _>>(),
            directives.simple.into_iter().collect::<BTreeMap<_, _>>(),
        );
        assert_eq!((found_qualified, found_simple), (owned(qualified), owned(simple)));
    }
}
