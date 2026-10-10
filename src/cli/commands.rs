use clap::{Arg, Command};

/// Builds the top-level CLI command and its subcommands.
pub fn build_cli() -> Command {
    Command::new("devinit")
        .version("0.0.2")
        .author("Gorkiy")
        .about("Devinit CLI tool")
        .arg_required_else_help(true)
        .subcommand(build_add_cli())
        .subcommand(build_list_cli())
        .subcommand(build_get_cli())
        .subcommand(build_license_cli())
        .subcommand(build_login_cli())
        .subcommand(build_compile_cli())
        .subcommand(build_test_cli())
}

/// Builds the service-add command.
fn build_add_cli() -> Command {
    Command::new("add")
        .about("Adding dependency")
        .arg(
            Arg::new("service")
                .help("Importing service command")
                .required(true)
                .value_parser(clap::value_parser!(String))
                .index(1),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .help("Default Port"),
        )
        .arg(
            Arg::new("version")
                .short('v')
                .long("version")
                .help("Specific version of service"),
        )
}

/// Builds the service-list command.
fn build_list_cli() -> Command {
    Command::new("list")
        .about("List of services")
        .arg(
            Arg::new("type")
                .short('t')
                .long("type")
                .help("Type of services"),
        )
        .arg(
            Arg::new("page")
                .short('p')
                .long("page")
                .help("Page of lists"),
        )
}

/// Builds the configuration-get command.
fn build_get_cli() -> Command {
    Command::new("get")
        .about("Importing services repo like space")
        .arg(
            Arg::new("service_url")
                .help("Get service from repo")
                .required(true)
                .index(1),
        )
}

/// Builds the license command.
fn build_license_cli() -> Command {
    Command::new("license").about("Lincensing project").arg(
        Arg::new("license_type")
            .help("license type")
            .required(true)
            .index(1),
    )
}

/// Builds the login command.
fn build_login_cli() -> Command {
    Command::new("login").about("Autentication").arg(
        Arg::new("login_url")
            .help("login via url")
            .required(true)
            .index(1),
    )
}

/// Builds the compile command.
fn build_compile_cli() -> Command {
    Command::new("compile").about("validate and generate compile.yaml")
}

/// Builds the test command.
fn build_test_cli() -> Command {
    Command::new("test")
        .about("testing")
        .arg(Arg::new("tester").help("test command"))
}
