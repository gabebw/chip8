use std::path::PathBuf;
use structopt::clap::AppSettings;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(global_settings(&[AppSettings::VersionlessSubcommands]))]
pub struct Arguments {
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
}
