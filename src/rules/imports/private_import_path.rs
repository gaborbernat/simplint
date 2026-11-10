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
    let same_stem = module == stem || module == format!("_{stem}");
    if level == 1 {
        return same_stem
            && package.is_empty()
            && walker
                .checker
                .path
                .strip_prefix(&walker.checker.settings.project.root)
                .is_ok_and(|path| !path.starts_with("tests"));
    }
    if level != 0 {
        return false;
    }
    let root = &walker.checker.settings.project.root;
    let relative = walker
        .checker
        .path
        .strip_prefix(root)
        .unwrap_or(walker.checker.path)
        .parent()
        .expect("checked file has a parent");
    let relative = relative.strip_prefix("tests").or_else(|_| relative.strip_prefix("test")).unwrap_or(relative);
    let relative = relative.strip_prefix("src").unwrap_or(relative);
    let package = PathBuf::from_iter(package.split('.').filter(|part| !part.is_empty()));
    let source = package.join(module);
    let mirrored = (same_stem && relative == package) || relative.starts_with(&source);
    let without_top = package.components().skip(1).collect::<PathBuf>();
    let mirrored_without_top = package.components().count() > 0
        && ((same_stem && relative == without_top) || relative.starts_with(without_top.join(module)));
    if !mirrored && !mirrored_without_top {
        return false;
    }
    [root.join(&source), root.join("src").join(source)]
        .iter()
        .any(|file| file.with_extension("py").is_file() || file.with_extension("pyi").is_file())
}

fn private_count(path: &str) -> usize {
    path.split('.').filter(|part| Visibility::of(part) == Visibility::Private).count()
}

fn report(walker: &Walker<'_, '_>, path: &str, range: TextRange) {
    let message = if walker.checker.is_test_module() {
        format!("Private import `{path}` in a test; only a test mirroring its private source file may import names")
    } else {
        format!("Private import `{path}`; import names from one private module with a direct relative path")
    };
    walker.checker.report(Rule::PrivateImportPath, range, message);
}
