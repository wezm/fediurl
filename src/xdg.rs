use eyre::WrapErr;
use simple_eyre::eyre;

pub fn new() -> eyre::Result<xdg::BaseDirectories> {
    xdg::BaseDirectories::with_prefix("fediurl")
        .wrap_err("unable to determine home directory of current user")
}
