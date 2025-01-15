// SPDX-License-Identifier: GPL-3

use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use clap::{Parser, ArgAction};
use cosmic::iced;
use crate::programs::ProgramSettings;

mod app;
mod config;
mod i18n;
mod programs;

#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Args {
    #[clap(long, short, action=ArgAction::SetFalse)]
    launch_only: bool,
}

fn main() -> cosmic::iced::Result {
    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    // Enable localizations to be applied.
    i18n::init(&requested_languages);

    let mut home_dir = std::env::home_dir().unwrap_or(PathBuf::from("/home/"));
    home_dir.push(".config");
    home_dir.push("cosmic-startup.ron");
    let mut selected_programs: Vec<ProgramSettings> = Vec::new();

    match fs::exists(&home_dir) {
        Ok(_) => selected_programs = ron::from_str(fs::read_to_string(&home_dir).unwrap().as_str()).unwrap(),
        Err(_) => {}
    }

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default().size_limits(
        cosmic::iced::Limits::NONE
            .min_width(360.0)
            .min_height(180.0),
    );

    let args = Args::parse();

    return if !args.launch_only {
        for program in selected_programs {
            let mut program_chunks = program.exec.split(" ");
            let program_name = program_chunks.next().unwrap();

            let base_args = shell_words::split(&*program_chunks.filter(|s| !s.starts_with("%")).collect::<Vec<&str>>().join(" ")).unwrap();

            println!("{}", program_name);
            println!("{}", shell_words::join(&base_args));

            let mut cmd = std::process::Command::new(program_name);
            cmd.args(base_args);

            for arg in &program.args {
                cmd.arg(arg);
            }
            let _ = cmd.spawn();
        }
        iced::Result::Ok(())
    } else {
        // Starts the application's event loop with `()` as the application's flags.
        cosmic::app::run::<app::AppModel>(settings, selected_programs)
    }
}
