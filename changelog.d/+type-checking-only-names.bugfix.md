`missing-from-all` (SMP401) no longer asks to list a name bound only under `if TYPE_CHECKING:`, since listing it makes
`from module import *` fail at run time.
