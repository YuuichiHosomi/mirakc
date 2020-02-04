mod broadcaster;
mod chunk_stream;
mod clock_synchronizer;
mod command_util;
mod config;
mod datetime_ext;
mod eit_feeder;
mod epg;
mod error;
mod job;
mod models;
mod mpeg_ts_stream;
mod service_scanner;
mod tokio_snippet;
mod tuner;
mod web;

use std::env;

use cfg_if;
use clap;
use pretty_env_logger;

use crate::error::Error;

cfg_if::cfg_if! {
    if #[cfg(feature = "use-jemalloc")] {
        use jemallocator::Jemalloc;
        #[global_allocator]
        static GLOBAL: Jemalloc = Jemalloc;
    } else if #[cfg(feature = "use-mimalloc")] {
        use mimalloc::MiMalloc;
        #[global_allocator]
        static GLOBAL: MiMalloc = MiMalloc;
    } else if #[cfg(feature = "use-tcmalloc")] {
        use tcmalloc::TCMalloc;
        #[global_allocator]
        static GLOBAL: TCMalloc = TCMalloc;
    }
}

#[actix_rt::main]
async fn main() -> Result<(), Error> {
    let args = clap::App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .about(clap::crate_description!())
        .arg(clap::Arg::with_name("config")
             .short("c")
             .long("config")
             .takes_value(true)
             .value_name("FILE")
             .env("MIRAKC_CONFIG")
             .help("Path to a configuration file in a YAML format")
             .long_help(
                 "Path to a configuration file in a YAML format.\n\
                  \n\
                  The MIRAKC_CONFIG environment variable is used if this \
                  option is not specified.  Its value has to be an absolute \
                  path.\n\
                  \n\
                  See README.md for details of the YAML format."))
        .get_matches();

    pretty_env_logger::init_timed();

    let config_path = args.value_of("config").expect(
        "--config option or MIRAKC_CONFIG environment must be specified");

    let config = config::load(config_path);

    tuner::start(config.clone());
    eit_feeder::start(config.clone());
    job::start(config.clone());
    epg::start(config.clone());
    web::serve(config.clone()).await?;

    Ok(())
}
