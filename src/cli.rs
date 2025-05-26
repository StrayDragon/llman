use crate::config::ENV_CONFIG_DIR;
use crate::error::Result;
use crate::prompt::PromptCommand;
use clap::{Arg, ArgAction, ArgMatches, Command};
use clap::{crate_name, crate_version};

pub struct Cli {
    command: Command,
}

impl Cli {
    pub fn new() -> Self {
        let command = Command::new(crate_name!())
            .version(crate_version!())
            .about(t!("cli.description").to_string())
            .long_about(t!("cli.description").to_string())
            .subcommand_required(true)
            .arg_required_else_help(true)
            .arg(
                Arg::new("config-dir")
                    .long("config-dir")
                    .value_name("DIR")
                    .help(t!("cli.config_dir_help").to_string())
                    .env(ENV_CONFIG_DIR)
                    .global(true),
            )
            .arg(
                Arg::new("log-level")
                    .long("log-level")
                    .value_name("LEVEL")
                    .help(t!("cli.log_level_help").to_string())
                    .value_parser(["DEBUG", "INFO", "WARN", "ERROR"])
                    .global(true),
            )
            .arg(
                Arg::new("verbose")
                    .short('v')
                    .long("verbose")
                    .help(t!("cli.verbose_help").to_string())
                    .action(ArgAction::Count)
                    .global(true),
            )
            .subcommand(
                Command::new("prompt")
                    .alias("rule")
                    .about(t!("prompt.about").to_string())
                    .subcommand_required(true)
                    .arg_required_else_help(true)
                    .subcommand(
                        Command::new("gen")
                            .about(t!("prompt.gen.about").to_string())
                            .arg(
                                Arg::new("interactive")
                                    .short('i')
                                    .long("interactive")
                                    .help(t!("prompt.gen.interactive_help").to_string())
                                    .action(ArgAction::SetTrue),
                            )
                            .arg(
                                Arg::new("app")
                                    .long("app")
                                    .value_name("APP_NAME")
                                    .help(t!("prompt.gen.app_help").to_string())
                                    .required_unless_present("interactive"),
                            )
                            .arg(
                                Arg::new("template")
                                    .long("template")
                                    .value_name("TEMPLATE_NAME")
                                    .help(t!("prompt.gen.template_help").to_string())
                                    .required_unless_present("interactive"),
                            )
                            .arg(
                                Arg::new("name")
                                    .long("name")
                                    .value_name("RULE_NAME")
                                    .help(t!("prompt.gen.name_help").to_string()),
                            )
                            .arg(
                                Arg::new("force")
                                    .long("force")
                                    .help(t!("prompt.gen.force_help").to_string())
                                    .action(ArgAction::SetTrue),
                            ),
                    )
                    .subcommand(
                        Command::new("list")
                            .about(t!("prompt.list.about").to_string())
                            .arg(
                                Arg::new("app")
                                    .long("app")
                                    .value_name("APP_NAME")
                                    .help(t!("prompt.list.app_help").to_string()),
                            ),
                    )
                    .subcommand(
                        Command::new("upsert")
                            .about(t!("prompt.upsert.about").to_string())
                            .arg(
                                Arg::new("app")
                                    .long("app")
                                    .value_name("APP_NAME")
                                    .help(t!("prompt.upsert.app_help").to_string())
                                    .required(true),
                            )
                            .arg(
                                Arg::new("name")
                                    .long("name")
                                    .value_name("RULE_NAME")
                                    .help(t!("prompt.upsert.name_help").to_string())
                                    .required(true),
                            )
                            .group(
                                clap::ArgGroup::new("content_source")
                                    .required(true)
                                    .args(["content", "file"]),
                            )
                            .arg(
                                Arg::new("content")
                                    .long("content")
                                    .value_name("TEXT")
                                    .help(t!("prompt.upsert.content_help").to_string()),
                            )
                            .arg(
                                Arg::new("file")
                                    .long("file")
                                    .value_name("PATH")
                                    .help(t!("prompt.upsert.file_help").to_string()),
                            ),
                    )
                    .subcommand(
                        Command::new("rm")
                            .about(t!("prompt.rm.about").to_string())
                            .arg(
                                Arg::new("app")
                                    .long("app")
                                    .value_name("APP_NAME")
                                    .help(t!("prompt.rm.app_help").to_string())
                                    .required(true),
                            )
                            .arg(
                                Arg::new("name")
                                    .long("name")
                                    .value_name("RULE_NAME")
                                    .help(t!("prompt.rm.name_help").to_string())
                                    .required(true),
                            ),
                    ),
            );

        Self { command }
    }

    pub fn run(&self) -> Result<()> {
        let matches = self.command.clone().get_matches();

        self.handle_global_options(&matches);

        match matches.subcommand() {
            Some(("prompt", prompt_matches)) | Some(("rule", prompt_matches)) => {
                self.handle_prompt_command(prompt_matches)
            }
            _ => {
                unreachable!("{}", t!("errors.subcommand_required"))
            }
        }
    }

    fn handle_global_options(&self, matches: &ArgMatches) {
        if let Some(log_level) = matches.get_one::<String>("log-level") {
            eprintln!("{}", t!("messages.log_level_set", level = log_level));
        }

        let verbose_count = matches.get_count("verbose");
        if verbose_count > 0 {
            eprintln!("{}", t!("messages.verbose_level", level = verbose_count));
        }
    }

    fn handle_prompt_command(&self, matches: &ArgMatches) -> Result<()> {
        let config_dir = matches.get_one::<String>("config-dir").map(|s| s.as_str());
        let prompt_cmd = PromptCommand::with_config_dir(config_dir)?;

        match matches.subcommand() {
            Some(("gen", gen_matches)) => {
                if gen_matches.get_flag("interactive") {
                    prompt_cmd.generate_interactive()
                } else {
                    let app = gen_matches.get_one::<String>("app").unwrap();
                    let template = gen_matches.get_one::<String>("template").unwrap();
                    let force = gen_matches.get_flag("force");

                    prompt_cmd.generate_rules(app, template, force)
                }
            }
            Some(("list", list_matches)) => {
                let app = list_matches.get_one::<String>("app").map(|s| s.as_str());
                prompt_cmd.list_rules(app)
            }
            Some(("upsert", upsert_matches)) => {
                let app = upsert_matches.get_one::<String>("app").unwrap();
                let name = upsert_matches.get_one::<String>("name").unwrap();
                let content = upsert_matches
                    .get_one::<String>("content")
                    .map(|s| s.as_str());
                let file = upsert_matches.get_one::<String>("file").map(|s| s.as_str());

                prompt_cmd.upsert_rule(app, name, content, file)
            }
            Some(("rm", rm_matches)) => {
                let app = rm_matches.get_one::<String>("app").unwrap();
                let name = rm_matches.get_one::<String>("name").unwrap();

                prompt_cmd.remove_rule(app, name)
            }
            _ => {
                unreachable!("{}", t!("errors.unknown_prompt_subcommand"))
            }
        }
    }
}
