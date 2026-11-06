use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::{Expr, PythonVersion};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;

enum Replacement {
    // The new name does the same work, so only the attribute name changes; the fix is unsafe where the new name's
    // value differs in type or `repr`.
    Rename(&'static str, Applicability),
    // The new name differs in a way that needs a review of each use.
    Review(&'static str),
}

struct Alias {
    module: &'static str,
    name: &'static str,
    // The first version that warns about the name or no longer has it.
    since: PythonVersion,
    replacement: Replacement,
}

const ALIASES: &[Alias] = &[
    Alias {
        module: "threading",
        name: "currentThread",
        since: PythonVersion::PY310,
        replacement: Replacement::Rename("current_thread", Applicability::Safe),
    },
    Alias {
        module: "threading",
        name: "activeCount",
        since: PythonVersion::PY310,
        replacement: Replacement::Rename("active_count", Applicability::Safe),
    },
    Alias {
        module: "calendar",
        name: "January",
        since: PythonVersion::PY312,
        replacement: Replacement::Rename("JANUARY", Applicability::Unsafe),
    },
    Alias {
        module: "calendar",
        name: "February",
        since: PythonVersion::PY312,
        replacement: Replacement::Rename("FEBRUARY", Applicability::Unsafe),
    },
    Alias {
        module: "configparser",
        name: "SafeConfigParser",
        since: PythonVersion::PY312,
        replacement: Replacement::Rename("ConfigParser", Applicability::Safe),
    },
    Alias {
        module: "pkgutil",
        name: "find_loader",
        since: PythonVersion::PY312,
        replacement: Replacement::Review("importlib.util.find_spec"),
    },
    Alias {
        module: "pkgutil",
        name: "get_loader",
        since: PythonVersion::PY312,
        replacement: Replacement::Review("importlib.util.find_spec"),
    },
    Alias {
        module: "unittest",
        name: "findTestCases",
        since: PythonVersion::PY311,
        replacement: Replacement::Review("unittest.TestLoader().loadTestsFromModule"),
    },
    Alias {
        module: "unittest",
        name: "makeSuite",
        since: PythonVersion::PY311,
        replacement: Replacement::Review("unittest.TestLoader().loadTestsFromTestCase"),
    },
    Alias {
        module: "unittest",
        name: "getTestCaseNames",
        since: PythonVersion::PY311,
        replacement: Replacement::Review("unittest.TestLoader().getTestCaseNames"),
    },
    Alias { module: "glob", name: "glob0", since: PythonVersion::PY313, replacement: Replacement::Review("glob.glob") },
    Alias { module: "glob", name: "glob1", since: PythonVersion::PY313, replacement: Replacement::Review("glob.glob") },
];

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    // An assignment or `del` patches the old name for code that still reads it, so only reads are reported.
    let Expr::Attribute(attribute) = expr else {
        return;
    };
    if !attribute.ctx.is_load() {
        return;
    }
    let target = checker.settings.from_ruff.target_version;
    // The name is compared first, so an attribute no alias names never builds the file's semantic model.
    let Some(alias) = ALIASES.iter().find(|alias| {
        alias.name == attribute.attr.as_str()
            && target >= alias.since
            && checker.semantic().resolves_to(expr, &[alias.module, alias.name])
    }) else {
        return;
    };
    let (fix, replacement, title) = match alias.replacement {
        Replacement::Rename(new, applicability) => {
            let replacement = format!("{}.{new}", alias.module);
            let fix =
                Fix::applicable_edit(Edit::range_replacement(new.to_owned(), attribute.attr.range()), applicability);
            (Some(fix), replacement.clone(), format!("Use `{replacement}`"))
        }
        Replacement::Review(new) => (None, new.to_owned(), format!("Use `{new}`")),
    };
    let message = format!(
        "`{}.{}` is deprecated or removed from Python {}; `{replacement}` replaces it",
        alias.module, alias.name, alias.since
    );
    checker.report_fix(Rule::DeprecatedStdlibAlias, expr.range(), message, fix, title);
}
