use std::path::{Path, PathBuf};

use itertools::Itertools;
use ruff_python_ast::identifier::Identifier;
use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::{Ranged, TextRange};

use ruff_diagnostics::{Applicability, Edit, Fix};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::edits::{fix_of, keep_debug_text};
use crate::rules::rename::appears_in_strings;
use crate::syntax::{Visibility, is_lazy};

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let caller = walker.checker.module_path();
    let file = walker.checker.path;
    if let Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) = stmt {
        let module = stmt.as_import_from_stmt().and_then(|from| from.module.as_ref()).map(ast::Identifier::as_str);
        for alias in names {
            private_alias(walker, module, alias);
        }
    }
    match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) => {
            for alias in names.iter().filter(|alias| private_count(alias.name.as_str()) > 0) {
                if !walker.checker.semantic().reported_by_plc2701(alias.identifier()) {
                    report(walker, 0, alias.name.as_str(), alias.name.range(), || {
                        module_hint(alias.name.as_str(), caller)
                    });
                }
            }
        }
        Stmt::ImportFrom(ast::StmtImportFrom { module, level, names, .. }) => {
            let path = module.as_ref().map_or("", ast::Identifier::as_str);
            // typeshed documents `_typeshed` as the module that stubs and `TYPE_CHECKING` blocks import from.
            if *level == 0 && path.split('.').next() == Some("_typeshed") {
                return;
            }
            let private = private_count(path);
            let paired = private > 0 && paired_test_module(walker, path, *level);
            let direct = *level == 1 && !path.contains('.');
            if private > 0 && !paired && (!direct || walker.checker.is_test_module()) {
                if !names.iter().any(|alias| walker.checker.semantic().reported_by_plc2701(alias.identifier())) {
                    report(walker, *level, path, module.as_ref().map_or(stmt.range(), Ranged::range), || {
                        import_hint(*level, path, names, is_lazy(stmt), caller)
                    });
                }
                return;
            }
            for alias in names.iter().filter(|alias| alias.name.as_str() != "*") {
                if *level == 0 && path == "os" && alias.name.as_str() == "_exit" {
                    continue;
                }
                // `_` is gettext's translation function by convention, not a private name.
                if alias.name.as_str() != "_"
                    && private_count(alias.name.as_str()) > 0
                    && !walker.checker.semantic().reported_by_plc2701(alias.identifier())
                {
                    report(walker, 0, alias.name.as_str(), alias.name.range(), || {
                        alias_hint(*level, path, alias.name.as_str(), file)
                    });
                }
            }
        }
        _ => {}
    }
}

fn private_alias(walker: &Walker<'_, '_>, module: Option<&str>, alias: &ast::Alias) {
    let checker = walker.checker;
    let Some(asname) = alias.asname.as_ref().map(ast::Identifier::as_str) else {
        return;
    };
    let name = alias.name.as_str();
    let qualified = module.map_or_else(|| name.to_owned(), |module| format!("{module}.{name}"));
    // Ruff's ICN001 checks aliases for modules with configured naming conventions.
    if asname == "_"
        || Visibility::of(asname) != Visibility::Private
        || private_count(name) > 0
        || checker.settings.from_ruff.import_aliases.contains_key(&qualified)
        // Ruff's preview adds these to its default conventions.
        || matches!(qualified.as_str(), "plotly.graph_objects" | "statsmodels.api")
    {
        return;
    }
    let public = last_segment_of(name);
    checker.report_fix(
        Rule::PrivateImportPath,
        alias.range(),
        format!("Public `{name}` imported as private `{asname}`; import it by its public name"),
        public_name_fix(checker, alias, asname, public),
        format!("Import `{public}` as `{public}`"),
    );
}

fn last_segment_of(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

// Dropping a dotted import's alias binds its root package. Function-local aliases are outside the module index.
fn public_name_fix(checker: &Checker, alias: &ast::Alias, asname: &str, public: &str) -> Option<Fix> {
    let project = checker.project();
    let ranges = checker.semantic().name_ranges(asname);
    if alias.name.as_str() != public
        || !ranges.iter().any(|range| alias.range().contains_range(*range))
        || checker.name_token_count(public) != 1
        || checker.name_token_count(asname) != ranges.len()
        || project.is_used_elsewhere(asname)
        || project.is_read_opaquely(asname)
        || project.is_computed_written()
        || appears_in_strings(checker, asname)
        || ranges.iter().any(|range| checker.state().in_source_text(checker, *range))
    {
        return None;
    }
    let renames = ranges
        .into_iter()
        .filter(|range| !alias.range().contains_range(*range))
        .map(|range| Edit::range_replacement(public.to_owned(), range));
    let edits = std::iter::once(Edit::range_replacement(public.to_owned(), alias.range())).chain(renames).collect();
    fix_of(keep_debug_text(checker, edits)?, Applicability::Unsafe)
}

fn private_count(path: &str) -> usize {
    path.split('.').filter(|part| Visibility::of(part) == Visibility::Private).count()
}

fn report(walker: &Walker<'_, '_>, level: u32, path: &str, range: TextRange, hint: impl FnOnce() -> String) {
    let message = if walker.checker.is_test_module() {
        format!("Private import `{path}` in a test; only a test mirroring its private source file may import names")
    } else {
        format!("Private import `{}{path}`; {}", ".".repeat(level as usize), hint())
    };
    walker.checker.report(Rule::PrivateImportPath, range, message);
}

fn import_hint(level: u32, path: &str, names: &[ast::Alias], lazy: bool, caller: Option<&[String]>) -> String {
    let imported: Vec<&ast::Alias> = names.iter().filter(|alias| alias.name.as_str() != "*").collect();
    if imported.is_empty() {
        return star_hint(level, path);
    }
    let segments: Vec<&str> = path.split('.').collect();
    let module = format!("{}{path}", ".".repeat(level as usize));
    let along = along(&module, &segments);
    let lazy_keyword = if lazy { "lazy " } else { "" };
    let pronoun = if imported.len() == 1 { "it" } else { "them" };
    let exported = imported.iter().map(|alias| alias.name.as_str()).join(", ");
    let bound = imported.iter().map(|alias| bound_name(alias)).join(", ");
    let private = imported
        .iter()
        .filter(|alias| private_count(alias.name.as_str()) > 0)
        .map(|alias| alias.name.as_str())
        .join(", ");
    match (level, path.split_once('.')) {
        (1, Some((first, _))) if !private.is_empty() => format!(
            "rename `{private}` to public names in `{module}`, re-export {pronoun} in {along} and import {pronoun} from \
             `.{first}`"
        ),
        (1, Some((first, _))) => format!(
            "re-export `{exported}` in {along} and import {pronoun} with `{lazy_keyword}from .{first} import {bound}`"
        ),
        (0, Some((package, rest))) if !rest.contains('.') => match (enters(caller, package), private.is_empty()) {
            (true, true) => format!(
                "inside `{package}` import {pronoun} with `{lazy_keyword}from .{rest} import {bound}`; elsewhere import \
                 {pronoun} from a public module that re-exports {pronoun}"
            ),
            (true, false) => format!(
                "rename `{private}` to public names in `{module}`; inside `{package}` import {pronoun} from `.{rest}`, \
                 elsewhere from a public module that re-exports {pronoun}"
            ),
            (false, true) => format!(
                "re-export `{exported}` in {along} and import {pronoun} with `{lazy_keyword}from {package} import {bound}`"
            ),
            (false, false) => format!(
                "rename `{private}` to public names in `{module}`, re-export {pronoun} in {along} and import {pronoun} \
                 from `{package}`"
            ),
        },
        (0, _) if !private.is_empty() => format!(
            "rename `{private}` to public names in `{module}`, and import {pronoun} from a public module that re-exports \
             {pronoun}"
        ),
        (0, _) => format!("import {pronoun} from a public module that re-exports {pronoun}"),
        _ if !private.is_empty() => format!(
            "rename `{private}` to public names in `{module}`, re-export {pronoun} in the `__init__.py` of the package \
             that holds {pronoun}, and import {pronoun} from there"
        ),
        _ => format!(
            "re-export `{exported}` in the `__init__.py` of the package that holds {pronoun}, and import {pronoun} from there"
        ),
    }
}

fn star_hint(level: u32, path: &str) -> String {
    match path.split_once('.') {
        Some((first, _)) if level == 1 => {
            let segments: Vec<&str> = path.split('.').collect();
            format!(
                "re-export the names you use in {} and import them from `.{first}`",
                along(&format!(".{path}"), &segments)
            )
        }
        _ => "re-export the names you use in the `__init__.py` of the package that holds them, and import them from there"
            .to_owned(),
    }
}

fn along(module: &str, segments: &[&str]) -> String {
    let packages: Vec<String> =
        (1..segments.len()).rev().map(|end| format!("`{}/__init__.py`", segments[..end].join("/"))).collect();
    if let [only] = packages.as_slice() {
        return only.clone();
    }
    format!("every package along `{module}`: {}", packages.join(", "))
}

fn bound_name(alias: &ast::Alias) -> String {
    match &alias.asname {
        Some(asname) => format!("{} as {}", alias.name.as_str(), asname.as_str()),
        None => alias.name.as_str().to_owned(),
    }
}

fn enters(caller: Option<&[String]>, package: &str) -> bool {
    matches!(caller, Some([top, _]) if top.as_str() == package)
}

fn alias_hint(level: u32, path: &str, name: &str, file: &Path) -> String {
    if path.is_empty() && level == 1 {
        format!("import the names you use from `.{name}`")
    } else if path.is_empty() {
        "re-export the names you use in the `__init__.py` of the package that holds them, and import them from there"
            .to_owned()
    } else if level == 1 && is_module(file, path, name) {
        let segments: Vec<&str> = path.split('.').chain([name]).collect();
        let module = format!(".{path}.{name}");
        format!("re-export the names you use in {} and import them from `.{path}`", along(&module, &segments))
    } else {
        let dots = ".".repeat(level as usize);
        format!("rename it to a public name in `{dots}{path}` and import it under that name")
    }
}

fn is_module(file: &Path, path: &str, name: &str) -> bool {
    let directory = file.parent().expect("checked file has a parent").join(PathBuf::from_iter(path.split('.')));
    directory.join(format!("{name}.py")).is_file()
        || directory.join(format!("{name}.pyi")).is_file()
        || directory.join(name).join("__init__.py").is_file()
}

fn module_hint(module: &str, caller: Option<&[String]>) -> String {
    match module.split_once('.') {
        Some((package, tail)) if !tail.contains('.') && enters(caller, package) => format!(
            "import the names you use from `.{tail}` inside `{package}`, or from a public module that re-exports them \
             elsewhere"
        ),
        Some((package, tail)) if !tail.contains('.') => {
            format!("re-export the names you use in `{package}/__init__.py` and import them from `{package}`")
        }
        _ => "import the names you use from a public module that re-exports them".to_owned(),
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
