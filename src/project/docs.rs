// The Python objects a project's documentation names: Sphinx cross-reference roles, autodoc directives and the
// options of directives that render a parser from a module's function, in reStructuredText and MyST Markdown. A
// definition the documentation names is used, even when no code of the run reaches it.
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

use filetime::FileTime;
use rkyv::util::AlignedVec;

use ignore::WalkBuilder;
use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_workspace::resolver::match_exclusion;
use rustc_hash::FxHashSet;

use crate::project::cache;

#[derive(Clone, Default, PartialEq)]
pub struct DocReferences {
    // The dotted names roles and directives give. Sphinx resolves a role against the page's module, its class or the
    // whole project, and `conf.py` may put any directory on `sys.path`, so any definition whose dotted name ends with
    // one may be the target.
    names: FxHashSet<String>,
    // `pkg.module.` of `automodule` and `pkg.module.Class.` of `autoclass` with `:members:`, whose members are all
    // documented, wherever the module's path starts.
    scopes: FxHashSet<String>,
}

const ROLES: [&str; 12] =
    ["func", "class", "meth", "attr", "data", "obj", "mod", "exc", "const", "type", "deco", "property"];

const OBJECTS: [&str; 10] = [
    "function",
    "class",
    "method",
    "attribute",
    "data",
    "exception",
    "decorator",
    "property",
    "classmethod",
    "staticmethod",
];

impl DocReferences {
    // Reads the `.rst` and `.md` files under each project root, skipping hidden directories and the paths Ruff
    // excludes from discovery there. The walk is the one discovery makes: `.gitignore` and `.ignore` files apply when
    // the settings respect them, and symlinks are not followed. With a cache directory, what a root's pages name is
    // kept there under the key `tree_key` gives, so a run where nothing under the root changed neither walks it nor
    // reads a page.
    pub fn collect<'a>(
        roots: impl IntoIterator<Item = (&'a Path, &'a ruff_workspace::Settings)>,
        cache_dir: Option<&Path>,
    ) -> Self {
        let mut references = Self::default();
        for (root, settings) in roots {
            let cache =
                cache_dir.map(|cache_dir| (cache::documentation_file(cache_dir, root), tree_key(root, settings)));
            if let Some(cached) = cache.as_ref().and_then(|(path, key)| Self::cached(path, *key)) {
                references.extend(cached);
                continue;
            }
            let mut found = Self::default();
            for path in pages(root, settings) {
                if let Ok(text) = fs::read_to_string(path) {
                    found.read(&text);
                }
            }
            if let Some((path, key)) = cache {
                found.store(&path, key);
            }
            references.extend(found);
        }
        references
    }

    // The references a cache file holds, when it was written under `key`.
    fn cached(path: &Path, key: u64) -> Option<Self> {
        let mut bytes = AlignedVec::<16>::new();
        bytes.extend_from_reader(&mut fs::File::open(path).ok()?).ok()?;
        let cached = rkyv::from_bytes::<CachedDocs, rkyv::rancor::Error>(&bytes).ok()?;
        (cached.key == key)
            .then(|| Self { names: cached.names.into_iter().collect(), scopes: cached.scopes.into_iter().collect() })
    }

    // Keeping the cache is best effort: a run that cannot write it reads the pages again next time.
    fn store(&self, path: &Path, key: u64) {
        let cached = CachedDocs {
            key,
            names: self.names.iter().cloned().collect(),
            scopes: self.scopes.iter().cloned().collect(),
        };
        let written = rkyv::to_bytes::<rkyv::rancor::Error>(&cached)
            .map_err(anyhow::Error::from)
            .and_then(|bytes| cache::write_atomically(path, &bytes));
        if let Err(error) = written {
            log::error!("Failed to write the documentation cache {}: {error:?}", path.display());
        }
    }

    fn extend(&mut self, other: Self) {
        self.names.extend(other.names);
        self.scopes.extend(other.scopes);
    }

    fn read(&mut self, text: &str) {
        let mut open: Option<Directive> = None;
        for line in text.lines() {
            let trimmed = line.trim();
            if let Some((name, argument)) = directive(trimmed) {
                if let Some(done) = open.replace(Directive { name, argument, options: Vec::new() }) {
                    done.record(self);
                }
            } else if let (Some(directive), Some(option)) = (open.as_mut(), option(trimmed)) {
                directive.options.push(option);
            } else if let Some(done) = open.take() {
                done.record(self);
            }
            self.names.extend(role_targets(line));
        }
        if let Some(done) = open {
            done.record(self);
        }
    }

    // Whether the documentation names the definition with the dotted name `qualified`, such as `pkg.module.Class.name`.
    // When the run cannot tell the module's package, `qualified` starts at the module's own name, and a documented name
    // that ends with it, under whichever package, matches too.
    pub fn covers(&self, qualified: &str, anchored: bool) -> bool {
        let mut suffixes =
            std::iter::once(qualified).chain(qualified.match_indices('.').map(|(at, _)| &qualified[at + 1..]));
        let below = |outer: &str, inner: &str| outer == inner || outer.ends_with(&format!(".{inner}"));
        suffixes.any(|suffix| {
            self.names.contains(suffix) || self.scopes.iter().any(|scope| suffix.starts_with(scope.as_str()))
        }) || (!anchored
            && (self.names.iter().any(|name| below(name, qualified))
                || qualified
                    .match_indices('.')
                    .any(|(at, _)| self.scopes.iter().any(|scope| below(scope, &qualified[..=at])))))
    }
}

// What a root's pages name, as the cache keeps it, with the key of the pages it was read from.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct CachedDocs {
    key: u64,
    names: Vec<String>,
    scopes: Vec<String>,
}

// The `.rst` and `.md` pages under a root the walk reaches.
fn pages(root: &Path, settings: &ruff_workspace::Settings) -> Vec<PathBuf> {
    let exclude = settings.file_resolver.exclude.clone();
    let extend_exclude = settings.file_resolver.extend_exclude.clone();
    let walk = WalkBuilder::new(root)
        .standard_filters(settings.file_resolver.respect_gitignore)
        .hidden(false)
        .filter_entry(move |entry| {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default();
            let hidden = entry.depth() > 0 && entry.file_name().to_str().is_some_and(|name| name.starts_with('.'));
            !hidden && !match_exclusion(path, name, &exclude) && !match_exclusion(path, name, &extend_exclude)
        })
        .build();
    walk.filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|file_type| file_type.is_file()) && is_page(entry.path()))
        // A markup file past 8 MiB is generated output rather than prose someone wrote, and reading it whole costs
        // memory for nothing.
        .filter(|entry| entry.metadata().is_ok_and(|metadata| metadata.len() <= 8 << 20))
        .map(ignore::DirEntry::into_path)
        .collect()
}

fn is_page(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rst" || extension == "md")
}

// A key of what the walk's outcome under a root depends on, read without building the matchers of its ignore files,
// which cost more than listing the directories: the modification time of each directory the walk may enter, which an
// added, removed or renamed entry moves, the modification time and size of each page and each ignore file, whose edits
// leave their directory's time as it was, those of the ignore files above the root and the global one, which the walk
// reads too, and the walk's settings. A directory an ignore file hides is listed all the same, so the key covers at
// least what the walk reads.
fn tree_key(root: &Path, settings: &ruff_workspace::Settings) -> u64 {
    let resolver = &settings.file_resolver;
    let mut hasher = CacheKeyHasher::new();
    resolver.cache_key(&mut hasher);
    for ancestor in root.ancestors() {
        for name in [".gitignore", ".ignore", ".git/info/exclude"] {
            stat_key(&ancestor.join(name), &mut hasher);
        }
    }
    if let Some(global) = ignore::gitignore::gitconfig_excludes_path() {
        stat_key(&global, &mut hasher);
    }
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        stat_key(&directory, &mut hasher);
        let mut entries: Vec<fs::DirEntry> =
            fs::read_dir(&directory).into_iter().flatten().filter_map(Result::ok).collect();
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let (path, name) = (entry.path(), entry.file_name());
            let Ok(file_type) = entry.file_type() else { continue };
            if file_type.is_dir() {
                let hidden = name.to_str().is_some_and(|name| name.starts_with('.'));
                if !hidden
                    && !match_exclusion(&path, &name, &resolver.exclude)
                    && !match_exclusion(&path, &name, &resolver.extend_exclude)
                {
                    pending.push(path);
                }
            } else if file_type.is_file() && (is_page(&path) || name == ".gitignore" || name == ".ignore") {
                stat_key(&path, &mut hasher);
            }
        }
    }
    hasher.finish()
}

// A path with its modification time and size, or with neither when it does not exist.
fn stat_key(path: &Path, hasher: &mut CacheKeyHasher) {
    path.cache_key(hasher);
    let stat = fs::metadata(path).ok().map(|metadata| {
        let modified = FileTime::from_last_modification_time(&metadata);
        (modified.unix_seconds(), modified.nanoseconds(), metadata.len())
    });
    stat.cache_key(hasher);
}

impl CacheKey for DocReferences {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        for names in [&self.names, &self.scopes] {
            let mut sorted: Vec<&String> = names.iter().collect();
            sorted.sort_unstable();
            sorted.cache_key(state);
        }
    }
}

// A directive and the options read under it so far.
struct Directive<'t> {
    name: &'t str,
    argument: &'t str,
    options: Vec<(&'t str, &'t str)>,
}

impl Directive<'_> {
    // The names a directive documents. A name relative to the page's `currentmodule` needs no resolving, since names
    // match by their dotted suffix.
    fn record(self, references: &mut DocReferences) {
        let value = |key: &str| self.options.iter().find(|(option, _)| *option == key).map(|(_, value)| *value);
        let target = self.argument.split(['(', ' ']).next().unwrap_or_default().to_owned();
        let name = self.name.strip_prefix("py:").unwrap_or(self.name);
        let object = name.strip_prefix("auto").unwrap_or(name);
        if matches!(object, "module" | "class" | "exception") && name.starts_with("auto") {
            // `:members:` alone documents every member, and with a list only those it names.
            match value("members") {
                Some("") => _ = references.scopes.insert(format!("{target}.")),
                Some(listed) => references.names.extend(
                    listed
                        .split(',')
                        .map(str::trim)
                        .filter(|member| !member.is_empty())
                        .map(|member| format!("{target}.{member}")),
                ),
                None => {}
            }
            references.names.insert(target);
        } else if OBJECTS.contains(&object) {
            references.names.insert(target);
        }
        // sphinx-argparse and sphinx-argparse-cli render the parser a module's function builds.
        if let Some(module) = value("module")
            && let Some(function) = value("func")
        {
            references.names.insert(format!("{module}.{function}"));
        }
        if let Some(path) = value("ref") {
            references.names.insert(path.to_owned());
        }
    }
}

// `.. name:: argument` in reStructuredText, or ```` ```{name} argument ```` and `:::{name} argument` in MyST.
fn directive(line: &str) -> Option<(&str, &str)> {
    if let Some(rest) = line.strip_prefix(".. ") {
        let (name, argument) = rest.split_once("::")?;
        return (!name.contains(' ')).then(|| (name, argument.trim()));
    }
    let rest = line.trim_start_matches(['`', ':']).strip_prefix('{').filter(|_| line.starts_with(['`', ':']))?;
    let (name, argument) = rest.split_once('}')?;
    Some((name, argument.trim()))
}

// `:key: value`, the option lines under a directive.
fn option(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.strip_prefix(':')?.split_once(':')?;
    (!key.is_empty() && !key.contains(' ')).then(|| (key, value.trim()))
}

// The targets of the line's Python roles: `:func:`name``, `:py:meth:`~pkg.Class.name``, `:class:`Title <pkg.Name>``,
// and MyST's `{py:func}`name``. A leading `.` searches every module, which the relative match already does.
fn role_targets(line: &str) -> Vec<String> {
    let parts: Vec<&str> = line.split('`').collect();
    (1..parts.len())
        .step_by(2)
        .filter_map(|at| {
            let (before, content) = (parts[at - 1], parts[at]);
            let role = match before.strip_suffix(':') {
                Some(rst) => rst.rsplit(':').next().unwrap_or_default(),
                None => before.strip_suffix('}')?.rsplit_once('{')?.1,
            };
            ROLES.contains(&role.strip_prefix("py:").unwrap_or(role)).then_some(())?;
            let target = content.rsplit_once('<').map_or(content, |(_, inner)| inner.trim_end_matches('>'));
            let target = target.trim_start_matches(['~', '!', '.']).trim_end_matches("()");
            (!target.is_empty()).then(|| target.to_owned())
        })
        .collect()
}
