use std::cell::RefCell;

use rustc_hash::{FxHashMap, FxHashSet};

use super::FileId;
use super::reach::starred_by_importer;
use super::resolve::{ClassBase, ClassKey, ModuleFacts};
use crate::project::facts::ImportBinding;

// A file and a name of it, as `Reexports::home` walks the imports that re-export a class.
type Node<'n> = (FileId, &'n str);

// The file and name each import binds, by the importing file and the name it binds.
type Bound<'b> = FxHashMap<FileId, FxHashMap<&'b str, Vec<(FileId, &'b str)>>>;

// The imports of a run, built once, and the files each file star-imports.
pub(super) struct Reexports<'b> {
    bound: Bound<'b>,
    pub(super) starred: FxHashMap<FileId, Vec<FileId>>,
    acyclic: FxHashSet<FileId>,
}

impl<'b> Reexports<'b> {
    pub(super) fn new(bindings: &[(FileId, FileId, &'b ImportBinding)], stars: &[(FileId, FileId)]) -> Self {
        let mut bound: Bound<'b> = FxHashMap::default();
        for (importer, source, binding) in bindings {
            bound
                .entry(*importer)
                .or_default()
                .entry(binding.bound.as_str())
                .or_default()
                .push((*source, binding.name.as_str()));
        }
        let starred = starred_by_importer(stars);
        let acyclic = acyclic_order(
            bindings.iter().map(|(importer, source, _)| (*importer, *source)).chain(stars.iter().copied()),
        )
        .into_iter()
        .collect();
        Self { bound, starred, acyclic }
    }

    // The class a name of a file is, following the imports that re-export it from the file that defines it. `is_class`
    // tells the classes of the run apart from the names they bind. The memo is kept per closure, since two predicates
    // can answer the same name differently.
    pub(super) fn home<'s>(
        &'s self,
        is_class: impl Fn(&ClassKey) -> bool + 's,
    ) -> impl Fn(FileId, &str) -> Option<ClassKey> + 's {
        let Self { bound, starred, acyclic } = self;
        let known: RefCell<FxHashMap<(FileId, String), Option<ClassKey>>> = RefCell::default();
        move |file: FileId, name: &str| -> Option<ClassKey> {
            let mut pending: Vec<(Node, Option<Node>)> = vec![((file, name), None)];
            let mut parents: FxHashMap<Node, Option<Node>> = FxHashMap::default();
            let mut found = None;
            while let Some((node, parent)) = pending.pop() {
                if parents.contains_key(&node) {
                    continue;
                }
                parents.insert(node, parent);
                let key = (node.0, node.1.to_owned());
                let cached = if acyclic.contains(&node.0) { known.borrow().get(&key).cloned() } else { None };
                match cached {
                    Some(Some(class)) => {
                        found = Some((node, class));
                        break;
                    }
                    Some(None) => continue,
                    None if is_class(&key) => {
                        found = Some((node, key));
                        break;
                    }
                    None => {}
                }
                let (file, name) = node;
                for (source, original) in bound.get(&file).and_then(|names| names.get(name)).into_iter().flatten() {
                    pending.push(((*source, *original), Some(node)));
                }
                pending.extend(starred.get(&file).into_iter().flatten().map(|source| ((*source, name), Some(node))));
            }
            match &found {
                Some((node, class)) => {
                    let mut known = known.borrow_mut();
                    let mut at = Some(*node);
                    while let Some(member) = at {
                        if acyclic.contains(&member.0) {
                            known.insert((member.0, member.1.to_owned()), Some(class.clone()));
                        }
                        at = parents[&member];
                    }
                }
                None => {
                    known.borrow_mut().extend(
                        parents
                            .keys()
                            .filter(|(file, _)| acyclic.contains(file))
                            .map(|&(file, name)| ((file, name.to_owned()), None)),
                    );
                }
            }
            found.map(|(_, class)| class)
        }
    }
}

// The files whose imports reach no cycle, each listed after the files it imports. The class memo covers only these: a
// walk through a cycle skips the files on its own path, so from a file above a cycle it can reach a different class than
// a walk started at that file would.
pub(super) fn acyclic_order(edges: impl IntoIterator<Item = (FileId, FileId)>) -> Vec<FileId> {
    let mut imports: FxHashMap<FileId, usize> = FxHashMap::default();
    let mut importers: FxHashMap<FileId, Vec<FileId>> = FxHashMap::default();
    for (importer, source) in edges {
        *imports.entry(importer).or_default() += 1;
        importers.entry(source).or_default().push(importer);
    }
    let mut ready: Vec<FileId> = importers.keys().filter(|file| !imports.contains_key(file)).copied().collect();
    let mut order = Vec::new();
    while let Some(file) = ready.pop() {
        order.push(file);
        for importer in importers.get(&file).into_iter().flatten() {
            let degree = imports.get_mut(importer).expect("an importer imports a file");
            *degree -= 1;
            if *degree == 0 {
                ready.push(*importer);
            }
        }
    }
    order
}

// The classes of a run, each with its bases and methods. A base that resolves to a class of the run is a parent of
// the class, and the class a child of that parent; any other base is outside the run, by the module and name it is
// imported by.
pub(super) struct ClassGraph<'m> {
    pub(super) classes: FxHashMap<ClassKey, (&'m [ClassBase], &'m [String])>,
    pub(super) parents: FxHashMap<ClassKey, Vec<ClassKey>>,
    children: FxHashMap<ClassKey, Vec<ClassKey>>,
    outside: FxHashMap<ClassKey, Vec<(&'m str, &'m str)>>,
}

impl<'m> ClassGraph<'m> {
    pub(super) fn of(modules: &'m FxHashMap<FileId, ModuleFacts<FileId>>, reexports: &Reexports) -> Self {
        let mut classes: FxHashMap<ClassKey, (&'m [ClassBase], &'m [String])> = FxHashMap::default();
        for (id, module) in modules {
            for (name, bases, methods) in &module.classes {
                classes.insert((*id, name.clone()), (bases.as_slice(), methods.as_slice()));
            }
        }
        let mut parents: FxHashMap<ClassKey, Vec<ClassKey>> = FxHashMap::default();
        let mut children: FxHashMap<ClassKey, Vec<ClassKey>> = FxHashMap::default();
        let mut outside: FxHashMap<ClassKey, Vec<(&'m str, &'m str)>> = FxHashMap::default();
        let home = reexports.home(|key| classes.contains_key(key));
        for (key, (bases, _)) in &classes {
            for (file, module, name) in *bases {
                match file.and_then(|file| home(file, name)) {
                    Some(parent) => {
                        children.entry(parent.clone()).or_default().push(key.clone());
                        parents.entry(key.clone()).or_default().push(parent);
                    }
                    None => outside.entry(key.clone()).or_default().push((module.as_str(), name.as_str())),
                }
            }
        }
        drop(home);
        Self { classes, parents, children, outside }
    }

    // The class a name of a file is, among the classes of the run.
    pub(super) fn home<'s>(&'s self, reexports: &'s Reexports<'_>) -> impl Fn(FileId, &str) -> Option<ClassKey> + 's {
        reexports.home(|key| self.classes.contains_key(key))
    }

    // The classes with a base outside the run that `is_root` accepts, and every class that derives from one of them.
    pub(super) fn descending_from(&self, is_root: impl Fn(&str, &str) -> bool) -> FxHashSet<&ClassKey> {
        let mut pending: Vec<&ClassKey> = self
            .outside
            .iter()
            .filter(|(_, bases)| bases.iter().any(|&(module, name)| is_root(module, name)))
            .map(|(key, _)| key)
            .collect();
        let mut found = FxHashSet::default();
        while let Some(key) = pending.pop() {
            if found.insert(key) {
                pending.extend(self.children.get(key).into_iter().flatten());
            }
        }
        found
    }
}
