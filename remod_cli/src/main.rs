use clap::{Parser as ClapParser, Subcommand};
use glob::glob;
use remod_config::Config;
use remod_core::display_name::DisplayName;
use remod_core::storybook::Storybook;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

#[derive(ClapParser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Set a custom config file to process modifications
    #[arg(short, long, value_name = ".remodrc")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialise a config for you to work with
    Init,
    /// Add `displayName` property to all components based on your config file
    DisplayNames {
        #[command(subcommand)]
        command: Option<DisplayNamesSubCommands>,
    },
    /// Create Stories for components. Creates a story file if doesn't exist
    Stories,
}

#[derive(Subcommand, Debug, Clone)]
enum DisplayNamesSubCommands {
    /// Add display name property to the components
    Add {
        /// Set prefix to add to the display names for the components. This option overrides the config
        #[arg(short, long, value_name = "prefix")]
        prefix: Option<String>,
    },
    /// Remove display name property from the components.
    Remove,
    /// Rename display name prefix to. Use this instead of removing and adding
    Rename {
        /// Set prefix to be replaced. This option overrides the config
        #[arg(short, long, value_name = "prefix")]
        prefix: Option<String>,
    },
}
fn main() {
    let cli = Cli::parse();

    if let Some(config) = cli.config.as_deref() {
        println!("Using the config provided by you {}", config.display());
    } else {
        println!("Using default config \'.remodrc\' at root of the project");
    }
    let remodrc = read_to_string(Path::new(".remodrc"));
    let (files, config) = match remodrc {
        Ok(contents) => {
            let config = Config::default().parse_config(contents);
            let glob_pattern = Path::new(config.root_dir.as_str()).join(&config.glob);
            (
                glob(
                    glob_pattern
                        .as_os_str()
                        .to_str()
                        .unwrap_or_else(|| "**/*/.tsx"),
                )
                .expect("Failed to load matching files.Check your glob pattern"),
                config,
            )
        }
        Err(_) => {
            let files =
                glob(&Config::default().glob).expect("failed to load any matching glob patterns");
            (files, Config::default())
        }
    };
    match cli.command {
        Some(cmd) => match cmd {
            Commands::Init => {
                println!("Yet to be implemented");
            }
            Commands::DisplayNames {
                command: sub_command,
            } => {
                let mut display_names = DisplayName {
                    modified: 0,
                    ignored: 0,
                    total: 0,
                    ..Default::default()
                };
                match sub_command {
                    Some(ref cmd) => match cmd {
                        DisplayNamesSubCommands::Add { prefix } => {
                            let final_prefix = match prefix {
                                Some(p) => p,
                                None => match &config.display_name_prefix {
                                    Some(p) => p,
                                    None => "",
                                },
                            };
                            display_names.prefix = final_prefix.to_string();
                            display_names.add_display_name(files, &config);
                        }
                        DisplayNamesSubCommands::Remove => {
                            display_names.remove_display_name(files, &config);
                        }
                        DisplayNamesSubCommands::Rename { prefix } => match prefix {
                            Some(prefix) => {
                                display_names.prefix = prefix.to_string();
                                display_names.rename_display_names(files, &config);
                            }
                            None => {
                                println!("Cannot proceed without prefix argument. Run `remod display-name rename -p PREFIX`");
                            }
                        },
                    },
                    None => {
                        println!("Please specify a sub command for display names to perform any modifications. Run `remod display-names -h to see the list of available options and subcommands for display names`")
                    }
                }
                display_names.display_stats();
            }
            Commands::Stories => {
                let mut storybook = Storybook::default();
                storybook.emit_story_files(files, &config);
                storybook.display_stats();
            }
        },
        None => {
            println!("Cannot run remod without any commands run `remod -h` to see the list of available commands");
        }
    }
}
