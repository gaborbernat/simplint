use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::relative_path;

pub fn import_cycle(checker: &Checker) {
    let project = checker.project();
    let Some(cycle) = project.import_cycle() else {
        return;
    };
    let (closing, kept) = cycle.chain().split_last().expect("a cycle closes on the file that reports it");
    let mut chain: Vec<String> = kept.iter().map(|&id| relative_path(project.path_of(id))).collect();
    if cycle.omitted() > 0 {
        chain.push(format!("... ({} more)", cycle.omitted()));
    }
    chain.push(relative_path(project.path_of(*closing)));
    checker.report(
        Rule::ImportCycle,
        cycle.range(),
        format!(
            "This import closes a cycle of runtime imports, `{}`; import inside the function that needs it",
            chain.join(" -> ")
        ),
    );
}
