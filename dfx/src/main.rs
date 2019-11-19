use clap::{App, AppSettings, Arg};
use slog::{error, Drain};

mod commands;
mod config;
mod lib;
mod util;

use crate::commands::CliCommand;
use crate::config::dfinity::Config;
use crate::lib::env::{
    BinaryCacheEnv, BinaryResolverEnv, ClientEnv, GlobalEnvironment, InProjectEnvironment,
    LoggerEnv, PlatformEnv, ProjectConfigEnv, VersionEnv,
};
use crate::lib::error::*;
use crate::lib::message::UserMessage;

fn cli<T>(env: &T) -> App<'_, '_>
where
    T: VersionEnv,
{
    App::new("dfx")
        .about("The DFINITY Executor.")
        .version(env.get_version().as_str())
        .global_setting(AppSettings::ColoredHelp)
        .subcommands(
            commands::builtin()
                .into_iter()
                .map(|x: CliCommand<InProjectEnvironment>| x.get_subcommand().clone()),
        )
        .arg(
            Arg::with_name("verbose")
                .long("version")
                .short("v")
                .takes_value(false)
                .multiple(true) // Each one increase the log level.
                .help(UserMessage::Verbose.to_str()),
        )
}

fn exec<T>(env: &T, args: &clap::ArgMatches<'_>, cli: &App<'_, '_>) -> DfxResult
where
    T: BinaryCacheEnv
        + BinaryResolverEnv
        + ClientEnv
        + LoggerEnv
        + PlatformEnv
        + ProjectConfigEnv
        + VersionEnv,
{
    let (name, subcommand_args) = match args.subcommand() {
        (name, Some(args)) => (name, args),
        _ => {
            cli.write_help(&mut std::io::stderr())?;
            eprintln!();
            eprintln!();
            return Ok(());
        }
    };

    match commands::builtin()
        .into_iter()
        .find(|x| name == x.get_name())
    {
        Some(cmd) => cmd.execute(env, subcommand_args),
        _ => {
            cli.write_help(&mut std::io::stderr())?;
            eprintln!();
            eprintln!();
            Err(DfxError::UnknownCommand(name.to_owned()))
        }
    }
}

fn main() {
    //    let decorator = slog_term::TermDecorator::new().build();
    //    let drain = slog_term::CompactFormat::new(decorator).build().fuse();
    //    let plain = slog_term::PlainSyncDecorator::new(std::io::stderr());
    //    let drain = slog_term::CompactFormat::new(plain).build().fuse();
    let decorator = slog_term::PlainDecorator::new(std::io::stdout());
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    let logger = slog::Logger::root(drain, slog::o!());
    slog::info!(logger, "hello");
    let result = {
        if Config::from_current_dir().is_ok() {
            // Build the environment.
            let env = InProjectEnvironment::from_current_dir()
                .expect("Could not create an project environment object.")
                .with_logger(logger);
            let matches = cli(&env).get_matches();

            exec(&env, &matches, &(cli(&env)))
        } else {
            let env = GlobalEnvironment::from_current_dir()
                .expect("Could not create an global environment object.")
                .with_logger(logger);
            let matches = cli(&env).get_matches();

            exec(&env, &matches, &(cli(&env)))
        }
    };

    if let Err(err) = result {
        let decorator = slog_term::PlainDecorator::new(std::io::stderr());
        let drain = slog_term::CompactFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = slog::Logger::root(drain, slog::o!());
        match err {
            DfxError::BuildError(err) => {
                error!(logger, "Build failed. Reason:");
                error!(logger, "  {}", err);
            }
            DfxError::IdeError(msg) => {
                error!(
                    logger,
                    "The Motoko Language Server returned an error:\n{}", msg
                );
            }
            DfxError::UnknownCommand(command) => {
                error!(logger, "Unknown command: {}", command);
            }
            DfxError::ProjectExists => {
                error!(
                    logger,
                    "Cannot create a new project because the directory already exists."
                );
            }
            DfxError::CommandMustBeRunInAProject => {
                error!(
                    logger,
                    "Command must be run in a project directory (with a dfx.json file)."
                );
            }
            DfxError::ClientError(code, message) => {
                error!(logger, "Client error (code {}): {}", code, message);
            }
            DfxError::Unknown(err) => {
                error!(logger, "Unknown error: {}", err);
            }
            DfxError::ConfigPathDoesNotExist(config_path) => {
                error!(logger, "Config path does not exist: {}", config_path);
            }
            DfxError::InvalidArgument(e) => {
                error!(logger, "Invalid argument: {}", e);
            }
            DfxError::InvalidData(e) => {
                error!(logger, "Invalid data: {}", e);
            }
            err => {
                error!(logger, "An error occured:\n{:#?}", err);
            }
        }

        std::process::exit(255);
    }
}
