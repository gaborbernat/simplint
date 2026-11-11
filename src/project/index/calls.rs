use rustc_hash::{FxHashMap, FxHashSet};
use std::path::PathBuf;

use crate::project::FileFacts;
use crate::project::constructors::{FileUses, RunCalls};

use super::files::FileId;

#[expect(clippy::too_many_arguments, reason = "the run's inputs, which the index passes on")]
pub(super) fn run_calls(
    files: &[(FileId, &FileFacts)],
    sources: &FxHashMap<FileId, &FileFacts>,
    constructor_uses: &[FileUses],
    imported: &FxHashMap<FileId, FxHashSet<String>>,
    module_reads: &FxHashSet<String>,
    entry_names: Vec<(FileId, String)>,
    referenced: &FxHashMap<FileId, FxHashSet<String>>,
    library_api: &FxHashMap<FileId, FxHashMap<String, Option<PathBuf>>>,
) -> RunCalls {
    let dynamic: FxHashSet<&str> =
        files.iter().flat_map(|(_, facts)| facts.constructors.dynamic.iter().map(String::as_str)).collect();
    let mut calls = RunCalls::index(
        files.iter().map(|(id, facts)| (*id, facts.constructors.dataclasses.as_slice())),
        constructor_uses,
        imported,
        &dynamic,
        &sources.iter().map(|(file, facts)| (*file, facts.defined.iter().map(String::as_str).collect())).collect(),
        &dynamic.iter().copied().chain(module_reads.iter().map(String::as_str)).collect(),
        library_api,
    );
    hide_names(&mut calls, entry_names, referenced);
    method_loads(&mut calls, files);
    calls
}

// Entry points and dotted strings reach these names through calls simplint cannot follow.
fn hide_names(
    calls: &mut RunCalls,
    entry_names: Vec<(FileId, String)>,
    referenced: &FxHashMap<FileId, FxHashSet<String>>,
) {
    for (file, name) in entry_names {
        calls.mark_opaque(file, name);
    }
    for (file, names) in referenced {
        for name in names {
            calls.mark_opaque(*file, name.clone());
        }
    }
}

// Counted per member, so a member's calls are checked against every read.
fn method_loads(calls: &mut RunCalls, files: &[(FileId, &FileFacts)]) {
    let mut loads: FxHashMap<&str, u32> = FxHashMap::default();
    for (_, facts) in files {
        for (name, count) in &facts.attribute_loads {
            *loads.entry(name.as_str()).or_default() += count;
        }
    }
    for (id, facts) in files {
        let constructors = &facts.constructors;
        let members = constructors
            .methods
            .iter()
            .chain(&constructors.attributes)
            .chain(&facts.functions)
            .chain(facts.classes.iter().map(|class| &class.name));
        calls.set_method_loads(
            *id,
            members.map(|member| (member.clone(), loads.get(member.as_str()).copied().unwrap_or_default())).collect(),
        );
    }
}
