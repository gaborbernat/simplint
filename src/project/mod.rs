pub(crate) mod cache;
pub(crate) mod computed_reads;
pub(crate) mod constructors;
mod docs;
mod facts;
mod index;
pub(crate) mod receivers;

pub use docs::{AutoclassContent, DocReferences};
pub(crate) use facts::doctest_code;
pub use facts::{FileFacts, Parts, Signature};
pub use index::{Exposure, External, FileIndex, ImportResolver, IndexInputs, Project, SearchRoots};

// A one-shot CLI process exits immediately after writing its output. Watch mode and library callers need the memory
// back.
#[derive(Clone, Copy)]
pub(crate) enum Ending {
    Exit,
    Drop,
}

impl Ending {
    pub(crate) fn end<T>(self, value: T) {
        match self {
            Self::Exit => std::mem::forget(value),
            Self::Drop => drop(value),
        }
    }
}
