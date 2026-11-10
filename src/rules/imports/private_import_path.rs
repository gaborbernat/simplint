use std::path::PathBuf;

use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::syntax::Visibility;

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) => {
            for alias in names.iter().filter(|alias| private_count(alias.name.as_str()) > 0) {
                report(walker, alias.name.as_str(), alias.name.range());
            }
        }
        Stmt::ImportFrom(ast::StmtImportFrom { module, level, names, .. }) => {
            let path = module.as_ref().map_or("", ast::Identifier::as_str);
            let private = private_count(path);
            if private > 0 && paired_test_module(walker, path, *level) {
                return;
            }
            let direct = *level == 1 && !path.contains('.');
            if private > 0 && (!direct || walker.checker.is_test_module()) {
                report(walker, path, module.as_ref().map_or(stmt.range(), Ranged::range));
                return;
            }
            for alias in names.iter().filter(|alias| alias.name.as_str() != "*") {
                if *level == 0 && path == "os" && alias.name.as_str() == "_exit" {
                    continue;
                }
                let count = private + private_count(alias.name.as_str());
                if count > 0 && (!direct || count > 1 || path.is_empty()) {
                    report(walker, alias.name.as_str(), alias.name.range());
                }
            }
        }
        _ => {}
    }
}

fn paired_test_module(walker: &Walker<'_, '_>, path: &str, level: u32) -> bool {
    let Some(stem) = walker.checker.file_name().strip_prefix("test_").and_then(|name| name.strip_suffix(".py")) else {
        return false;
    };
    let (package, module) = path.rsplit_once('.').unwrap_or(("", path));
    if module != format!("_{stem}") {
        return false;
    }
    if level == 1 {
        return package.is_empty()
            && walker
                .checker
                .path
                .strip_prefix(&walker.checker.settings.project.root)
                .is_ok_and(|path| !path.starts_with("tests"));
    }
    if level != 0 {
        return false;
    }
    let package = PathBuf::from_iter(package.split('.'));
    walker.checker.path.parent().is_some_and(|parent| parent.ends_with(package))
}

fn private_count(path: &str) -> usize {
    path.split('.').filter(|part| Visibility::of(part) == Visibility::Private).count()
}

fn report(walker: &Walker<'_, '_>, path: &str, range: TextRange) {
    let message = if walker.checker.is_test_module() {
        format!("Private import `{path}` in a test; only `test_x.py` may import from its matching `_x.py`")
    } else {
        format!("Private import `{path}`; import names from one private module with a direct relative path")
    };
    walker.checker.report(Rule::PrivateImportPath, range, message);
}
