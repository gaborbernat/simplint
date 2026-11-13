use std::path::{Path, PathBuf};

use super::{AutoclassContent, Directives, DocReferences, Module, RootDocs};
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
    named: AutoclassContent,
    qualified: Vec<(String, Option<AutoclassContent>)>,
    simple: Vec<(String, Option<AutoclassContent>)>,
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
                    let directives = Directives {
                        qualified: root.qualified.into_iter().collect(),
                        simple: root.simple.into_iter().collect(),
                    };
                    let docs = RootDocs { default: root.default, named: root.named, directives };
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
                    named: docs.named,
                    qualified: docs
                        .directives
                        .qualified
                        .iter()
                        .map(|(name, content)| (name.clone(), *content))
                        .collect(),
                    simple: docs.directives.simple.iter().map(|(name, content)| (name.clone(), *content)).collect(),
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

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::DocReferences;

    #[test]
    fn corrupt_cache_is_a_miss() {
        let root = TempDir::new().expect("create a temporary directory");
        let path = root.path().join("docs");
        std::fs::write(&path, [0xff; 64]).expect("write the cache");
        assert!(DocReferences::cached(&path, 0).is_none());
    }

    #[test]
    fn missing_cache_is_a_miss() {
        let root = TempDir::new().expect("create a temporary directory");
        assert!(DocReferences::cached(&root.path().join("docs"), 0).is_none());
    }
}
