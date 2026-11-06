use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::{alias_binding, for_each_body, is_name};

const CODECS: [&str; 4] = ["gzip", "bz2", "lzma", "zlib"];

pub fn up_compression_modules(checker: &Checker) {
    // The `compression` package is where the new names are defined, so its own imports of the old names stay.
    if checker.module_path().and_then(<[String]>::first).is_some_and(|package| package == "compression") {
        return;
    }
    for_each_body(checker.body(), &mut |statements, owner| {
        // A guarded import is a fallback for the Pythons that lack the new name, so it stays as written.
        if matches!(owner, Some(Stmt::Try(_))) {
            return;
        }
        for stmt in statements {
            match stmt {
                Stmt::Import(import) => {
                    for alias in &import.names {
                        if !CODECS.contains(&alias.name.as_str()) {
                            continue;
                        }
                        // A statement that binds other names too would need splitting, so only a lone import is fixed.
                        let fix = (import.names.len() == 1
                            && all_re_exported(&alias.name, &attributes_of(checker.body(), alias_binding(alias))))
                        .then(|| {
                            let asname = alias.asname.as_ref().map_or_else(String::new, |name| format!(" as {name}"));
                            let replacement = format!("from compression import {}{asname}", alias.name);
                            Fix::unsafe_edit(Edit::range_replacement(replacement, stmt.range()))
                        });
                        report(checker, alias.name.range(), alias.name.as_str(), fix);
                    }
                }
                Stmt::ImportFrom(import @ ast::StmtImportFrom { module: Some(module), level: 0, .. }) => {
                    if !CODECS.contains(&module.as_str()) {
                        continue;
                    }
                    let names = import.names.iter().map(|alias| alias.name.as_str()).collect::<Vec<_>>();
                    let fix = all_re_exported(module, &names).then(|| {
                        Fix::unsafe_edit(Edit::range_replacement(format!("compression.{module}"), module.range()))
                    });
                    report(checker, module.range(), module.as_str(), fix);
                }
                _ => {}
            }
        }
    });
}

fn report(checker: &Checker, range: TextRange, codec: &str, fix: Option<Fix>) {
    checker.report_fix(
        Rule::CompressionModules,
        range,
        format!("`{codec}` is available as `compression.{codec}` from Python 3.14"),
        fix,
        format!("Import `compression.{codec}`"),
    );
}

// `compression.{codec}` runs `from {codec} import *`, so it lacks private names, the modules the codec imports, and for
// `gzip` every name outside its short `__all__`. `*` stands for a star import, which takes the same names.
fn all_re_exported(codec: &str, names: &[&str]) -> bool {
    names.iter().all(|name| {
        *name == "*"
            || !name.starts_with('_')
                && if codec == "gzip" {
                    ["BadGzipFile", "GzipFile", "open", "compress", "decompress"].contains(name)
                } else {
                    !["builtins", "io", "os"].contains(name)
                }
    })
}

// The attributes the file reads off `name` anywhere, whatever the name is bound to there.
fn attributes_of<'a>(body: &'a [Stmt], name: &str) -> Vec<&'a str> {
    let mut reads = AttributeReads { name, attributes: Vec::new() };
    reads.visit_body(body);
    reads.attributes
}

struct AttributeReads<'n, 'a> {
    name: &'n str,
    attributes: Vec<&'a str>,
}

impl<'a> Visitor<'a> for AttributeReads<'_, 'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Attribute(attribute) = expr
            && is_name(&attribute.value, self.name)
        {
            self.attributes.push(attribute.attr.as_str());
        }
        visitor::walk_expr(self, expr);
    }
}
