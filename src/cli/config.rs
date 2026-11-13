use anyhow::Result;

use crate::cli::args::{CheckCommand, GlobalConfigArgs};
use crate::run::{Resolved, choose_cache_dir, redact_extended};
use crate::settings::SettingsResolver;

// The configurations come from the command line, which names the configuration file and the cache directory, so they
// are resolved here and the run only reads them.
impl Resolved {
    pub(crate) fn new(cli: &CheckCommand, global_options: &GlobalConfigArgs) -> Result<Self> {
        let (config_arguments, arguments) = cli.partition(global_options)?;
        let settings_resolver = SettingsResolver::new(
            &arguments.cwd,
            arguments.config_file.as_deref(),
            arguments.isolated,
            arguments.overrides,
        )?;
        let pyproject_config = ruff::resolve::resolve(&config_arguments, cli.stdin_filename.as_deref())
            .map_err(|error| redact_extended(error, &settings_resolver.root.root))?;
        let cache_dir = choose_cache_dir(cli.cache_dir.is_some(), &pyproject_config);
        Ok(Self { config_arguments, settings_resolver, pyproject_config, cache_dir })
    }
}
