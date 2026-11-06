use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::relative_path;

pub fn import_cycle(checker: &Checker) {
    let Some(cycle) = checker.project().import_cycle() else {
        return;
    };
    let chain: Vec<String> = cycle.chain().iter().map(|path| relative_path(path)).collect();
    checker.report(
        Rule::ImportCycle,
        cycle.range(),
        format!(
            "This import closes a cycle of runtime imports, `{}`; import inside the function that needs it",
            chain.join(" -> ")
        ),
    );
}
