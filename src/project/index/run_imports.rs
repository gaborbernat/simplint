use std::path::Path;

use rustc_hash::{FxHashMap, FxHashSet};

use super::classes::Reexports;
use super::resolve::{ClassKey, ModuleFacts};
use super::{FileId, ImportResolver};
use crate::project::FileFacts;
use crate::project::facts::{Channel, ImportBinding, Signature};
use crate::project::receivers::module_parts;
use crate::syntax::Visibility;

// A resolved class maps to true and a function to false, including re-exports. Other names have no entry.
pub(super) type Members = FxHashMap<String, bool>;

#[derive(Default)]
pub(super) struct RunImports {
    pub(super) names: FxHashMap<FileId, Members>,
    pub(super) modules: FxHashMap<FileId, FxHashMap<String, FileId>>,
    pub(super) members: FxHashMap<FileId, Members>,
    pub(super) signatures: FxHashMap<FileId, FxHashMap<String, Signature>>,
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
    let mut signatures: FxHashMap<FileId, FxHashMap<String, Signature>> = FxHashMap::default();
    for (importer, source, binding) in bindings {
        bound.entry(*importer).or_default().push(binding.bound.as_str());
        let Some(key) = home(*source, &binding.name) else {
            continue;
        };
        let is_class = classes.contains(&key);
        names.entry(*importer).or_default().insert(binding.bound.clone(), is_class);
        // Several definitions may share a signature, for example in alternative version branches. Conflicting
        // signatures leave the call ambiguous.
        let mut shapes = modules
            .get(&key.0)
            .into_iter()
            .flat_map(|module| &module.signatures)
            .filter(|signature| signature.name == key.1);
        let found = shapes.next().filter(|first| shapes.all(|other| other == *first));
        if let Some(signature) = found.filter(|_| !is_class) {
            signatures.entry(*importer).or_default().insert(binding.bound.clone(), signature.clone());
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
    RunImports { names, modules, members, signatures }
}

pub(super) fn private_package_exports(
    indexed: &[(FileId, &FileFacts)],
    resolver: &ImportResolver,
    tests: &FxHashSet<FileId>,
    distributions: &FxHashMap<&Path, &Path>,
) -> FxHashMap<FileId, FxHashSet<String>> {
    let packages: FxHashMap<_, _> = indexed
        .iter()
        .filter_map(|(id, facts)| {
            let path = resolver.files.path(*id);
            let exports = facts.exports.as_deref().unwrap_or(&facts.conditional_exports);
            if !(path.ends_with("__init__.py") || path.ends_with("__init__.pyi")) || exports.is_empty() {
                return None;
            }
            let module = resolver.search.module_path(path).unwrap_or_default();
            let parts = module_parts(&module);
            let [_, family] = parts else { return None };
            if Visibility::of(family) != Visibility::Private {
                return None;
            }
            let root = path.ancestors().nth(parts.len()).expect("each module component comes from the path");
            Some((*id, (parts.join("."), root, exports, distributions[path])))
        })
        .collect();
    let mut allowed = FxHashMap::<_, FxHashSet<String>>::default();
    if packages.is_empty() {
        return allowed;
    }
    for (caller, _) in indexed {
        let path = resolver.files.path(*caller);
        for (target, names) in &resolver.modules[caller].names[Channel::Import].files {
            let Some((module, package, exports, distribution)) = packages.get(target) else { continue };
            if *distribution != distributions[path] || (!path.starts_with(package) && !tests.contains(caller)) {
                continue;
            }
            for name in names.iter().filter(|name| Visibility::of(name) == Visibility::Public && exports.contains(name))
            {
                allowed.entry(*caller).or_default().insert(format!("{module}.{name}"));
            }
        }
    }
    allowed
}
