use rustc_hash::{FxHashMap, FxHashSet};

use crate::project::FileFacts;
use crate::project::receivers::ANY_ATTRIBUTE;

use super::files::FileId;
use crate::syntax::last_dotted_segment;

#[derive(Default)]
pub(super) struct Writes {
    pub(super) on_class: FxHashMap<String, FxHashSet<String>>,
    pub(super) class_attributes: FxHashMap<String, FxHashSet<String>>,
    pub(super) untyped: FxHashSet<String>,
    pub(super) dict_stores: FxHashSet<String>,
    pub(super) unslottable_bases: FxHashSet<String>,
    pub(super) stateful_bases: FxHashSet<String>,
    pub(super) pickles_low_protocol: bool,
    pub(super) namesakes: FxHashMap<String, usize>,
}

impl Writes {
    pub(super) fn collect(files: &[(FileId, &FileFacts)]) -> Self {
        let mut writes = Self::default();
        let classes: FxHashSet<&str> =
            files.iter().flat_map(|(_, facts)| facts.attribute_writes.classes.iter().map(String::as_str)).collect();
        let mut bases: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
        for (class, base) in files.iter().flat_map(|(_, facts)| &facts.attribute_writes.class_bases) {
            bases.entry(class.as_str()).or_default().push(base.as_str());
        }
        for (_, facts) in files {
            let facts = &facts.attribute_writes;
            for (class, attribute) in &facts.class_attributes {
                let mut reached = vec![class.as_str()];
                let mut seen = FxHashSet::default();
                while let Some(reached_class) = reached.pop() {
                    if seen.insert(reached_class) {
                        writes.class_attributes.entry(reached_class.to_owned()).or_default().insert(attribute.clone());
                        reached.extend(bases.get(reached_class).into_iter().flatten().copied());
                    }
                }
            }
            for (class, attribute) in &facts.typed {
                writes.on_class.entry(class.clone()).or_default().insert(attribute.clone());
            }
            // An annotation naming a class the run does not define, such as `Any`, a protocol or a type alias, gives an
            // object of unknown class, whose writes the field checks read by name.
            for (class, attribute) in &facts.declared {
                if classes.contains(class.as_str()) {
                    writes.on_class.entry(class.clone()).or_default().insert(attribute.clone());
                } else if attribute != ANY_ATTRIBUTE {
                    writes.untyped.insert(attribute.clone());
                }
            }
            writes.untyped.extend(facts.untyped.iter().cloned());
            writes.dict_stores.extend(facts.dict_stores.iter().cloned());
            writes
                .dict_stores
                .extend(facts.declared_dict_stores.iter().filter(|class| classes.contains(class.as_str())).cloned());
            writes.unslottable_bases.extend(facts.unslottable_bases.iter().cloned());
            writes.stateful_bases.extend(facts.stateful_bases.iter().cloned());
            writes.pickles_low_protocol |= facts.pickles_low_protocol;
        }
        for class in writes.on_class.keys().chain(&writes.unslottable_bases) {
            *writes.namesakes.entry(last_dotted_segment(class).to_owned()).or_default() += 1;
        }
        writes
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use super::Writes;
    use crate::project::index::FileIds;
    use crate::project::{FileFacts, Parts};

    // A class attribute reaches every base once, though a diamond reaches the shared base on two paths; a computed name
    // written on an object of a class the run does not define names no field.
    #[test]
    fn class_attributes_reach_each_base_once() {
        let source = "from typing import Any\n\n\nclass Base:\n    pass\n\n\nclass Left(Base):\n    pass\n\n\n\
                      class Right(Base):\n    pass\n\n\nclass Both(Left, Right):\n    pass\n\n\nBoth.limit = 1\n\n\n\
                      def store(target: Any, name: str) -> None:\n    setattr(target, name, 1)\n";
        let path = Path::new("/project/shapes.py");
        let facts = FileFacts::collect(path, source, None, None, Parts::default()).expect("facts");
        let mut ids = Arc::new(FileIds::default());
        let writes = Writes::collect(&[(FileIds::intern(&mut ids, path), &facts)]);
        let mut reached: Vec<&str> = writes.class_attributes.keys().map(String::as_str).collect();
        reached.sort_unstable();
        assert_eq!(
            (reached, writes.untyped.contains("*")),
            (vec!["shapes.Base", "shapes.Both", "shapes.Left", "shapes.Right"], false)
        );
    }
}
