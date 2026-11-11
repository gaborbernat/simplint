use std::path::Path;

use super::{DocReferences, Module};
use crate::project::cache;

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct CachedDocs {
    key: u64,
    names: Vec<String>,
    scopes: Vec<String>,
    modules: Vec<Module>,
}

impl DocReferences {
    pub(super) fn cached(path: &Path, key: u64) -> Option<Self> {
        let bytes = cache::read_aligned(path).ok()?;
        let cached = rkyv::from_bytes::<CachedDocs, rkyv::rancor::Error>(&bytes).ok()?;
        (cached.key == key).then(|| Self {
            names: cached.names.into_iter().collect(),
            scopes: cached.scopes.into_iter().collect(),
            modules: cached.modules.into_iter().collect(),
        })
    }

    // Keeping the cache is best effort: a run that cannot write it reads the pages again next time.
    pub(super) fn store(&self, path: &Path, key: u64) {
        let cached = CachedDocs {
            key,
            names: self.names.iter().cloned().collect(),
            scopes: self.scopes.iter().cloned().collect(),
            modules: self.modules.iter().cloned().collect(),
        };
        let written = rkyv::to_bytes::<rkyv::rancor::Error>(&cached)
            .map_err(anyhow::Error::from)
            .and_then(|bytes| cache::write_atomically(path, &bytes));
        if let Err(error) = written {
            log::error!("Failed to write the documentation cache {}: {error:?}", path.display());
        }
    }
}
