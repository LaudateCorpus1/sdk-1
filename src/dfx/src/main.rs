use clap::{App, AppSettings};

mod commands;
mod config;
mod lib;
mod util;

use crate::commands::CliCommand;
use crate::config::dfinity::Config;
use crate::config::dfx_version;
use crate::lib::env::{
    BinaryCacheEnv, BinaryResolverEnv, ClientEnv, GlobalEnvironment, InProjectEnvironment,
    PlatformEnv, ProjectConfigEnv, VersionEnv,
};
use crate::lib::error::*;

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
}

fn exec<T>(env: &T, args: &clap::ArgMatches<'_>, cli: &App<'_, '_>) -> DfxResult
where
    T: BinaryCacheEnv + VersionEnv + BinaryResolverEnv + ClientEnv + PlatformEnv + ProjectConfigEnv,
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
    let result = {
        if Config::from_current_dir().is_ok() {
            // Build the environment.
            let env = InProjectEnvironment::from_current_dir()
                .expect("Could not create an project environment object.");

            // If we're not using the right version, forward the call to the cache dfx.
            if dfx_version() != env.get_version() {
                match crate::config::cache::call_cached_dfx(env.get_version()) {
                    Ok(status) => std::process::exit(status.code().unwrap_or(0)),
                    Err(e) => {
                        eprintln!("Error when trying to forward to project dfx:\n{:?}", e);
                        eprintln!("Installed executable: {}", dfx_version());
                        std::process::exit(1)
                    }
                };
            }

            let matches = cli(&env).get_matches();

            exec(&env, &matches, &(cli(&env)))
        } else {
            let env = GlobalEnvironment::from_current_dir()
                .expect("Could not create an global environment object.");
            let matches = cli(&env).get_matches();

            exec(&env, &matches, &(cli(&env)))
        }
    };

    if let Err(err) = result {
        match err {
            DfxError::BuildError(err) => {
                eprintln!("Build failed. Reason:");
                eprintln!("  {}", err);
            }
            DfxError::IdeError(msg) => {
                eprintln!("The Motoko Language Server returned an error:\n{}", msg);
            }
            DfxError::UnknownCommand(command) => {
                eprintln!("Unknown command: {}", command);
            }
            DfxError::ProjectExists => {
                eprintln!("Cannot create a new project because the directory already exists.");
            }
            DfxError::CommandMustBeRunInAProject => {
                eprintln!("Command must be run in a project directory (with a dfx.json file).");
            }
            DfxError::ClientError(code, message) => {
                eprintln!("Client error (code {}): {}", code, message);
            }
            DfxError::Unknown(err) => {
                eprintln!("Unknown error: {}", err);
            }
            DfxError::ConfigPathDoesNotExist(config_path) => {
                eprintln!("Config path does not exist: {}", config_path);
            }
            DfxError::InvalidArgument(e) => {
                eprintln!("Invalid argument: {}", e);
            }
            DfxError::InvalidData(e) => {
                eprintln!("Invalid data: {}", e);
            }
            DfxError::LanguageServerFromATerminal => {
                eprintln!("The `_language-service` command is meant to be run by editors to start a language service. You probably don't want to run it from a terminal.\nIf you _really_ want to, you can pass the --force-tty flag.");
            }
            err => {
                eprintln!("An error occured:\n{:#?}", err);
            }
        }

        std::process::exit(255);
    }
}