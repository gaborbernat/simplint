use std::path::{Path, PathBuf};

use super::{AutoclassContent, DocReferences, Module, RootDocs};
use crate::project::cache;

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct CachedDocs {
    key: u64,
    names: Vec<String>,
    scopes: Vec<String>,
    modules: Vec<Module>,
    roots: Vec<CachedRoot>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct CachedRoot {
    root: String,
    default: AutoclassContent,
    directives: Vec<(String, AutoclassContent)>,
}

impl DocReferences {
    // Any read failure or key mismatch is a miss: the pages are read again and the cache is written anew.
    pub(super) fn cached(path: &Path, key: u64) -> Option<Self> {
        let cached = rkyv::from_bytes::<CachedDocs, rkyv::rancor::Error>(&cache::read_aligned(path).ok()?).ok()?;
        (cached.key == key).then(|| Self {
            names: cached.names.into_iter().collect(),
            scopes: cached.scopes.into_iter().collect(),
            modules: cached.modules.into_iter().collect(),
            roots: cached
                .roots
                .into_iter()
                .map(|root| {
                    let docs = RootDocs { default: root.default, directives: root.directives.into_iter().collect() };
                    (PathBuf::from(root.root), docs)
                })
                .collect(),
        })
    }

    // Keeping the cache is best effort: a run that cannot write it reads the pages again next time.
    pub(super) fn store(&self, path: &Path, key: u64) {
        let cached = CachedDocs {
            key,
            names: self.names.iter().cloned().collect(),
            scopes: self.scopes.iter().cloned().collect(),
            modules: self.modules.iter().cloned().collect(),
            roots: self
                .roots
                .iter()
                .map(|(root, docs)| CachedRoot {
                    root: root.to_string_lossy().into_owned(),
                    default: docs.default,
                    directives: docs.directives.iter().map(|(name, content)| (name.clone(), *content)).collect(),
                })
                .collect(),
        };
        let written = rkyv::to_bytes::<rkyv::rancor::Error>(&cached)
            .map_err(anyhow::Error::from)
            .and_then(|bytes| cache::write_atomically(path, &bytes));
        if let Err(error) = written {
            log::error!("Failed to write the documentation cache {}: {error:?}", path.display());
        }
    }
}
