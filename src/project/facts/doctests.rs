use ruff_python_ast::helpers::resolve_imported_module_path;
use ruff_python_stdlib::identifiers::is_identifier;

use super::Channel;
use super::visitor::FactVisitor;

// The code of a docstring's `>>>` example lines, without the prompt.
pub(crate) fn doctest_code(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter_map(|line| line.trim_start().strip_prefix(">>>"))
}

// The identifiers of the `>>>` example lines of a docstring.
pub(super) fn doctest_names(text: &str) -> impl Iterator<Item = String> + '_ {
    doctest_code(text)
        .flat_map(|code| code.split(|char: char| !(char.is_alphanumeric() || char == '_')))
        .filter(|word| is_identifier(word))
        .map(str::to_owned)
}

// The level, module and names a `from module import name, other as alias` example imports.
pub(crate) fn from_import(code: &str) -> Option<(u32, &str, Vec<&str>)> {
    let (module, names) = code.trim_start().strip_prefix("from ")?.split_once(" import ")?;
    let module = module.trim();
    let name = module.trim_start_matches('.');
    let level = u32::try_from(module.len() - name.len()).ok()?;
    let names = names
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split(',')
        .filter_map(|name| name.split_whitespace().next())
        .filter(|name| *name != "*" && is_identifier(name))
        .collect();
    Some((level, name, names))
}

impl FactVisitor<'_> {
    // The imports and dotted names a docstring's examples make, which the run reads as it reads the code's own, so a
    // doctest's `from module import name` keeps `name` used.
    pub(super) fn doctests(&mut self, text: &str) {
        for code in doctest_code(text) {
            if let Some((level, module, names)) = from_import(code)
                && let Some(resolved) =
                    resolve_imported_module_path(level, (!module.is_empty()).then_some(module), self.module_path)
            {
                self.names[Channel::Import]
                    .entry(resolved.into_owned())
                    .or_default()
                    .extend(names.into_iter().map(str::to_owned));
            }
            // A doctest reads a name off a module, which no patch of the module is, but the call-site rules must see it.
            for path in code.split(|char: char| !(char.is_alphanumeric() || matches!(char, '_' | '.'))) {
                if path.contains('.') && path.split('.').all(is_identifier) {
                    Self::record_path(&mut self.names[Channel::Doctest], path);
                }
            }
        }
    }
}
