use clap_verbosity_flag::Verbosity;
use std::path::PathBuf;
use structopt::clap::AppSettings;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(global_settings(&[AppSettings::VersionlessSubcommands]))]
pub struct Arguments {
    #[structopt(flatten)]
    pub verbose: Verbosity,

    #[structopt(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(StructOpt)]
pub enum Subcommand {
    #[structopt(about = "Print instructions in this file")]
    Print {
        #[structopt(parse(from_os_str))]
        input_file_path: PathBuf,
    },
    #[structopt(about = "Trace the execution flow")]
    Trace {
        #[structopt(parse(from_os_str))]
        input_file_path: PathBuf,
    },
    #[structopt(about = "Run a program")]
    Run {
        #[structopt(parse(from_os_str))]
        input_file_path: PathBuf,
    },
}

pub fn install_logger(verbose: &mut Verbosity) {
    verbose.set_default(Some(log::Level::Warn));
    let level_filter = verbose.log_level().map(|l| l.to_level_filter());
    let mut logger = env_logger::Builder::new();
    logger.filter(None, level_filter.unwrap_or(log::LevelFilter::Warn));
    logger.format_module_path(false);
    logger.format_timestamp(None);
    logger.init();
}
