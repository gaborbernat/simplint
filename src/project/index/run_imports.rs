use rustc_hash::{FxHashMap, FxHashSet};

use super::FileId;
use super::classes::Reexports;
use super::resolve::{ClassKey, ModuleFacts};
use crate::project::facts::ImportBinding;

// A name the file only imports, assigns or leaves to a module `__getattr__` is absent, so nothing tells what it is.
pub(super) type Members = FxHashMap<String, bool>;

#[derive(Default)]
pub(super) struct RunImports {
    pub(super) names: FxHashMap<FileId, Members>,
    pub(super) modules: FxHashMap<FileId, FxHashMap<String, FileId>>,
    pub(super) members: FxHashMap<FileId, Members>,
}

pub(super) fn run_import_kinds(
    modules: &FxHashMap<FileId, ModuleFacts<FileId>>,
    bindings: &[(FileId, FileId, &ImportBinding)],
    reexports: &Reexports,
) -> RunImports {
    let mut classes: FxHashSet<ClassKey> = FxHashSet::default();
    let mut functions: FxHashSet<ClassKey> = FxHashSet::default();
    for (id, module) in modules {
        classes.extend(module.classes.iter().map(|(name, _, _)| (*id, name.clone())));
        functions.extend(module.functions.iter().map(|name| (*id, name.clone())));
    }
    let home = reexports.home(|key| classes.contains(key) || functions.contains(key));
    let defined = |file: FileId, name: &str| home(file, name).map(|key| classes.contains(&key));
    let mut bound: FxHashMap<FileId, Vec<&str>> = FxHashMap::default();
    let mut names: FxHashMap<FileId, Members> = FxHashMap::default();
    for (importer, source, binding) in bindings {
        bound.entry(*importer).or_default().push(binding.bound.as_str());
        if let Some(is_class) = defined(*source, &binding.name) {
            names.entry(*importer).or_default().insert(binding.bound.clone(), is_class);
        }
    }
    let members: FxHashMap<FileId, Members> = modules
        .iter()
        .map(|(id, module)| {
            let own = module
                .classes
                .iter()
                .map(|(name, _, _)| name.as_str())
                .chain(module.functions.iter().map(String::as_str))
                .chain(bound.get(id).into_iter().flatten().copied());
            let found = own.filter_map(|name| Some((name.to_owned(), defined(*id, name)?))).collect();
            (*id, found)
        })
        .collect();
    let modules: FxHashMap<FileId, FxHashMap<String, FileId>> = modules
        .iter()
        .map(|(importer, module)| {
            let found = module
                .modules
                .iter()
                .chain(&module.plain_modules)
                .filter(|(_, _, target)| members.contains_key(target))
                .map(|(alias, _, target)| (alias.clone(), *target))
                .collect();
            (*importer, found)
        })
        .collect();
    RunImports { names, modules, members }
}
