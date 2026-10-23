    # Devinit 
    CLI Tool which help reduce time to configuration to deployment.

    ## Документация

    Подробное описание архитектуры, файлов, функций, моделей, валидаторов,
    генераторов и текущих ограничений находится в
    [docs/PROJECT_DOCUMENTATION.md](docs/PROJECT_DOCUMENTATION.md).

    ## Exporting configs feature
    First of all, it will generate docker-compose.yaml and env files from 
    the web service through exporting configs.

    ## Compile feature
    Secondly, devinit will have feature for "compiling" specific file to generate configuration. For example, K8s and CI/CD configs

    Alghoritms of compiling.
    1) Read file
    2) Parse from yaml to struct
    3) Validate the values from struct
        1)  Resolve model
    4) Generating config
    5) Writing to files

    ### Reading file

    Devinit will find the file compile.yaml and read as a String


    ### Parse

    Parsing will parse the compile.yaml via serde_yaml crate to structure.
    And will have basic validation which included in parse structs

    ### Validating

    Validate feature will check and validates the fields.
    It will check for necessarily fields and existing of dependencies
    and for ports configuration, correctly specified dockerfile
    for application and are the specified targets supported?

    ### Resolving 

    Will be added later

    ### Generating

    generating files and directories

    ### Writing

    Writing in generated files.


    ## Loging feature

    With login feature user could authorize in web services (main or private). User could get private configurations from web seriveces.

    ## Importing feature

    User could import encrypted configurations from web services to secure secret data

    ## List feature

    In the future. List feature could show list of default services , as example famous services like PostgreSQL, Redis et al. and give oportunity to choose version of service.


    ## Install feature

    User could install services from web services to install specific configuration

