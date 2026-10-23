pub mod api {
    pub mod add_recipe;
    pub mod get_config;
    pub mod login;
}
pub mod cli {
    pub mod cli_logic;
    pub mod commands;
}
pub mod file_config {
    pub mod env_config;
    pub mod file_validator;
    pub mod yaml_config;
    pub mod yaml_writer;
}
pub mod models {
    pub mod cicd_struct;
    pub mod compile_struct;
    pub mod docker_compose_struct;
    pub mod env_struct;
    pub mod k8s_struct;
    pub mod resolver_struct;
    pub mod services_struct;
}

pub mod validator {
    pub mod cicd_validator;
    pub mod compile_validator;
    pub mod k8s_validator;
    pub mod services_validator;
}
pub mod resolved_spec {

    pub mod resolver;
}

pub mod docker {
    pub mod docker_finder;
}

pub mod errors;
pub mod generator;
