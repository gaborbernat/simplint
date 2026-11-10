use std::env;
use std::path::{Path, PathBuf};

use anyhow::Result;
use ruff_linter::warn_user;
use ruff_workspace::resolver::PyprojectConfig;

use crate::cli::args::{CheckCommand, GlobalConfigArgs};
use crate::project::cache;
use crate::run::{Resolved, redact_extended, within};
use crate::settings::SettingsResolver;

// The configurations come from the command line, which names the configuration file and the cache directory, so they
// are resolved here and the run only reads them.
impl Resolved {
    pub(crate) fn new(cli: &CheckCommand, global_options: &GlobalConfigArgs) -> Result<Self> {
        let (config_arguments, arguments) = cli.partition(global_options)?;
        let settings_resolver =
            SettingsResolver::new(arguments.config_file.as_deref(), arguments.isolated, arguments.overrides)?;
        let pyproject_config = ruff::resolve::resolve(&config_arguments, cli.stdin_filename.as_deref())
            .map_err(|error| redact_extended(error, &settings_resolver.root.root))?;
        let cache_dir = choose_cache_dir(cli, &pyproject_config);
        Ok(Self { config_arguments, settings_resolver, pyproject_config, cache_dir })
    }
}

// Ruff's cache directory, where simplint keeps its cache. A path from the command line or `RUFF_CACHE_DIR` is the user's
// choice and stands wherever it points. Otherwise the repository being linted picks it, through `cache-dir` or a link
// committed in place of the default, so it must stay inside the project root: the directory of the configuration file,
// or the working directory without one. A `cache-dir` that leaves the root falls back to Ruff's default there, and a
// default that leaves it too turns the cache off.
fn choose_cache_dir(cli: &CheckCommand, config: &PyprojectConfig) -> Option<PathBuf> {
    let configured = &config.settings.cache_dir;
    if cli.cache_dir.is_some() {
        return Some(configured.clone());
    }
    let root = match config.path.as_deref().and_then(Path::parent) {
        Some(parent) => parent.to_path_buf(),
        None => env::current_dir().ok()?,
    };
    let chosen = if within(configured, &root) { configured.clone() } else { ruff_cache::cache_dir(&root) };
    // The directory of this build is the deepest the cache creates, so checking it covers a link at any level above.
    let build = cache::documentation_file(&chosen, &root);
    if build.parent().is_some_and(|build| within(build, &root)) {
        return Some(chosen);
    }
    warn_user!("Not caching: the cache directory `{}` leads outside the project", chosen.display());
    None
}
