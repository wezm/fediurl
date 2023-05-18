use std::path::{Path, PathBuf};

use eyre::eyre;
use simple_eyre::eyre;

pub struct BaseDirs;

pub fn new() -> eyre::Result<BaseDirs> {
    Ok(BaseDirs)
}

impl BaseDirs {
    pub fn place_config_file<P: AsRef<Path>>(&self, path: P) -> eyre::Result<PathBuf> {
        ::dirs::config_dir()
            .ok_or_else(|| eyre!("unable to determine user config dir"))
            .map(|mut config| {
                config.push("open-in-mastodon"); // FIXME: better name
                config.push(path);
                config
            })
    }
}
