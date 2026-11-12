pub(crate) mod cache;
pub(crate) mod computed_reads;
pub(crate) mod constructors;
mod docs;
mod facts;
mod index;
pub(crate) mod receivers;

pub use docs::{AutoclassContent, DocReferences};
pub(crate) use facts::doctest_code;
pub use facts::{FileFacts, Parts};
pub use index::{Exposure, External, FileIndex, ImportResolver, IndexInputs, Project, SearchRoots};

// What a run does with the structures it built once its output is out. A one-shot CLI process exits next, so freeing
// them only delays the exit; watch mode and library callers go on, and drop them.
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
