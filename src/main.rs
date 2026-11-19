use ani_lib::{ErrorReport, ErrorVerbosity};
use clap::Parser;

mod bin_app;

#[tokio::main]
async fn main() {
    bin_app::log::init();
    let cli = bin_app::cli::Cli::parse();
    let verbosity = if cli.debug {
        ErrorVerbosity::Debug
    } else if cli.verbose {
        ErrorVerbosity::Verbose
    } else {
        ErrorVerbosity::Normal
    };

    if let Err(error) = bin_app::app::run(cli).await {
        let report = ErrorReport::from_error(&error);
        eprintln!("{}", report.render(verbosity));
        std::process::exit(1);
    }
}
