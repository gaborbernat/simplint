use ruff_python_ast::Operator;

use super::Kind;

pub(super) fn shared(kinds: impl IntoIterator<Item = Kind>) -> Option<Kind> {
    let mut kinds = kinds.into_iter();
    let first = kinds.next()?;
    kinds.all(|kind| kind == first).then_some(first)
}

pub(super) fn binary(op: Operator, sides: (Kind, Kind)) -> Option<Kind> {
    match (op, sides) {
        (Operator::Add, (left, right))
            if left == right
                && matches!(left, Kind::Str | Kind::Int | Kind::Bytes | Kind::StrItems | Kind::IntItems) =>
        {
            Some(left)
        }
        (Operator::Mult, (Kind::Str, Kind::Int) | (Kind::Int, Kind::Str)) => Some(Kind::Str),
        (
            Operator::Mult,
            (items @ (Kind::StrItems | Kind::IntItems), Kind::Int)
            | (Kind::Int, items @ (Kind::StrItems | Kind::IntItems)),
        ) => Some(items),
        (Operator::Mult | Operator::Sub | Operator::FloorDiv | Operator::Mod, (Kind::Int, Kind::Int)) => {
            Some(Kind::Int)
        }
        (Operator::Div, (Kind::Path, Kind::Str | Kind::Path)) => Some(Kind::Path),
        _ => None,
    }
}

pub(super) fn method(receiver: Kind, name: &str) -> Option<Kind> {
    const STR_METHODS: [&str; 21] = [
        "capitalize",
        "casefold",
        "center",
        "expandtabs",
        "format",
        "format_map",
        "join",
        "ljust",
        "lower",
        "lstrip",
        "removeprefix",
        "removesuffix",
        "replace",
        "rjust",
        "rstrip",
        "strip",
        "swapcase",
        "title",
        "translate",
        "upper",
        "zfill",
    ];
    match (receiver, name) {
        (Kind::Str, name) if STR_METHODS.contains(&name) => Some(Kind::Str),
        (Kind::Bytes, "decode" | "hex") | (Kind::Path, "as_posix" | "as_uri" | "read_text") => Some(Kind::Str),
        (Kind::Str, "partition" | "rpartition" | "rsplit" | "split" | "splitlines")
        | (Kind::StrItems, "keys" | "values") => Some(Kind::StrItems),
        (Kind::Str, "count" | "find" | "index" | "rfind" | "rindex") => Some(Kind::Int),
        (Kind::Str, "encode") => Some(Kind::Bytes),
        (
            Kind::Path,
            "absolute" | "expanduser" | "joinpath" | "relative_to" | "resolve" | "with_name" | "with_stem"
            | "with_suffix",
        ) => Some(Kind::Path),
        _ => None,
    }
}

pub(super) const PLATFORM_FUNCTIONS: [&str; 8] =
    ["machine", "node", "platform", "python_implementation", "python_version", "release", "system", "version"];

pub(super) const STRING_CONSTANTS: [&str; 9] = [
    "ascii_letters",
    "ascii_lowercase",
    "ascii_uppercase",
    "digits",
    "hexdigits",
    "octdigits",
    "printable",
    "punctuation",
    "whitespace",
];

pub(super) const OS_PATH_FUNCTIONS: [&str; 10] = [
    "abspath",
    "basename",
    "dirname",
    "expanduser",
    "expandvars",
    "join",
    "normcase",
    "normpath",
    "realpath",
    "relpath",
];

pub(super) const PATH_CLASSES: [&str; 6] =
    ["Path", "PosixPath", "PurePath", "PurePosixPath", "PureWindowsPath", "WindowsPath"];

pub(super) fn items(item: Kind) -> Option<Kind> {
    match item {
        Kind::Str => Some(Kind::StrItems),
        Kind::Int => Some(Kind::IntItems),
        _ => None,
    }
}

// Generic containers whose item type their parameters give; a mapping's keys and values must share it.
pub(super) const CONTAINERS: [&str; 22] = [
    "AbstractSet",
    "Collection",
    "Dict",
    "FrozenSet",
    "Iterable",
    "Iterator",
    "List",
    "Mapping",
    "MutableMapping",
    "MutableSequence",
    "MutableSet",
    "Sequence",
    "Set",
    "Tuple",
    "dict",
    "frozenset",
    "list",
    "set",
    "tuple",
    "Reversible",
    "KeysView",
    "ValuesView",
];
