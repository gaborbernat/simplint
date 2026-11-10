use std::path::{Path, PathBuf};

use anyhow::Result;
use ruff_workspace::resolver::PyprojectConfig;

use crate::cli::args::{CheckCommand, GlobalConfigArgs};
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

// Ruff's cache directory, where simplint keeps its cache. A `cache-dir` in a configuration file comes from the
// repository being linted, so it must stay inside the project root, the directory of that file; one that leaves it
// falls back to Ruff's default there. A path from the command line or `RUFF_CACHE_DIR` is the user's choice and stands
// wherever it points.
fn choose_cache_dir(cli: &CheckCommand, config: &PyprojectConfig) -> PathBuf {
    let configured = &config.settings.cache_dir;
    match config.path.as_deref().and_then(Path::parent) {
        Some(root) if cli.cache_dir.is_none() && !within(configured, root) => ruff_cache::cache_dir(root),
        _ => configured.clone(),
    }
}
