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
            let direct = *level == 1 && !path.contains('.');
            if private > 0 && !direct {
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

fn private_count(path: &str) -> usize {
    path.split('.').filter(|part| Visibility::of(part) == Visibility::Private).count()
}

fn report(walker: &Walker<'_, '_>, path: &str, range: TextRange) {
    walker.checker.report(
        Rule::PrivateImportPath,
        range,
        format!("Private import `{path}`; import names from one private module with a direct relative path"),
    );
}
