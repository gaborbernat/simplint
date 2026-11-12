use std::path::PathBuf;

use ruff_python_ast::identifier::Identifier;
use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::syntax::Visibility;

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) => {
            for alias in names.iter().filter(|alias| private_count(alias.name.as_str()) > 0) {
                if !walker.checker.semantic().reported_by_plc2701(alias.identifier()) {
                    report(walker, alias.name.as_str(), alias.name.range());
                }
            }
        }
        Stmt::ImportFrom(ast::StmtImportFrom { module, level, names, .. }) => {
            let path = module.as_ref().map_or("", ast::Identifier::as_str);
            let private = private_count(path);
            let paired = private > 0 && paired_test_module(walker, path, *level);
            let direct = *level == 1 && !path.contains('.');
            if private > 0 && !paired && (!direct || walker.checker.is_test_module()) {
                if !names.iter().any(|alias| walker.checker.semantic().reported_by_plc2701(alias.identifier())) {
                    report(walker, path, module.as_ref().map_or(stmt.range(), Ranged::range));
                }
                return;
            }
            for alias in names.iter().filter(|alias| alias.name.as_str() != "*") {
                if *level == 0 && path == "os" && alias.name.as_str() == "_exit" {
                    continue;
                }
                if private_count(alias.name.as_str()) > 0
                    && !walker.checker.semantic().reported_by_plc2701(alias.identifier())
                {
                    report_private_name(walker, alias.name.as_str(), alias.name.range());
                }
            }
        }
        _ => {}
    }
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

fn report_private_name(walker: &Walker<'_, '_>, name: &str, range: TextRange) {
    walker.checker.report(
        Rule::PrivateImportPath,
        range,
        format!("Private import `{name}`; import a public name from its owning module"),
    );
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
    let package_dir = PathBuf::from_iter(package.split('.').filter(|part| !part.is_empty()));
    let without_top = package_dir.components().skip(1).collect::<PathBuf>();
    let mirrored_without_top = package_dir.components().count() > 0
        && ((same_stem && relative == without_top) || relative.starts_with(without_top.join(module)));
    let source = package_dir.join(module);
    let mirrored = (same_stem && relative == package_dir) || relative.starts_with(&source);
    if !mirrored && !mirrored_without_top {
        return false;
    }
    [root.join(&source), root.join("src").join(source)]
        .iter()
        .any(|file| file.with_extension("py").is_file() || file.with_extension("pyi").is_file())
}
