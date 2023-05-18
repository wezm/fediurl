use std::process::ExitCode;
use std::{env, io};

use log::error;
use reqwest::Url;
use simple_eyre::eyre;

const FEDIURL_LOG: &str = "FEDIURL_LOG";

#[tokio::main]
async fn main() -> ExitCode {
    match try_main().await {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(report) => {
            error!("{:?}", report);
            ExitCode::FAILURE
        }
    }
}

async fn try_main() -> eyre::Result<bool> {
    simple_eyre::install()?;
    match env::var_os(FEDIURL_LOG) {
        None => env::set_var(FEDIURL_LOG, "info"),
        Some(_) => {}
    }
    pretty_env_logger::try_init_custom_env(FEDIURL_LOG)?;

    let args: Vec<_> = env::args().skip(1).collect();
    let args: Vec<_> = args.iter().map(|s| s.as_str()).collect();

    match args.as_slice() {
        ["auth", instance] => {
            let instance_url = instance.parse()?;
            fediurl::auth(instance_url).await?;
            Ok(true)
        }
        ["auth", ..] => {
            eprintln!("Usage: auth <instance url>");
            Ok(false)
        }
        [url] => {
            // TODO: Pass in config path if supplied
            let url = Url::parse(&url)?; // TODO: Add context
            let new_url = fediurl::rewrite_url(&url).await?;
            print_result(new_url.as_ref());
            Ok(true)
        }
        [] => {
            // TODO: Pass in config path if supplied
            // Read URL from stdin
            let mut line = String::new();
            io::stdin().read_line(&mut line)?;
            let url = Url::parse(&line)?; // TODO: Add context
            let new_url = fediurl::rewrite_url(&url).await?;
            print_result(new_url.as_ref());
            Ok(true)
        }
        _ => {
            eprintln!("unexpected arguments");
            Ok(false)
        }
    }
}

fn print_result(url: Option<&Url>) {
    match url {
        Some(url) => println!("{}", url),
        None => eprintln!("No match"),
    }
}
