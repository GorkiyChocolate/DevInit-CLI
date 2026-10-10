# DevInit: подробная документация

Документ описывает фактическое состояние проекта `devinit_cli_2` по исходному коду. Он рассчитан на пользователя, который хочет запустить CLI, и на разработчика, которому нужно понять назначение каждого файла, функции и этапа обработки.

## 1. Назначение проекта
DevInit — CLI на Rust для подготовки конфигурационных файлов из одного YAML-описания. Основной сценарий — команда `devinit compile`:

```text
compile.yaml
    -> проверка наличия и непустого содержимого
    -> чтение UTF-8
    -> десериализация YAML в CompileSpec
    -> валидация сервисов, Kubernetes и CI/CD
    -> проверка зависимостей между сервисами
    -> генерация целевых YAML-файлов
    -> атомарная запись результатов
```

Генерация выполняется только после успешной десериализации и полной валидации. Проект не запускает Docker, Kubernetes или CI/CD: он только создаёт конфигурационные файлы.

## 2. Быстрый запуск

Требуется установленный Rust и Cargo.

```bash
cargo build
cargo test
cargo run -- compile
```

`compile` всегда ищет файл `compile.yaml` в текущем рабочем каталоге. Для запуска уже собранного бинарного файла:

```bash
./target/debug/devinit_cli_2 compile
```

Основные команды справки:

```bash
cargo run -- --help
cargo run -- compile --help
```

## 3. Структура проекта

```text
Cargo.toml                         манифест пакета и зависимости
README.md                          краткое описание проекта
Instruction.md                     схема compile.yaml и примеры
TODO.md                            реализованные возможности и план развития
compile.yaml                       пример Compose + GitHub Actions
compile12.yaml                     пример только со встроенными сервисами
compile43.yaml                     пример Compose + Kubernetes + GitLab CI
resources/services.yaml            справочный список сервисов
src/lib.rs                         экспорт модулей библиотеки
src/main.rs                        бинарная точка входа
src/errors.rs                      типы ошибок
src/api/                            HTTP-клиент
src/cli/                            описание команд и маршрутизация CLI
src/file_config/                   чтение и запись конфигурационных файлов
src/generator/                     генерация целевых файлов
src/models/                        структуры YAML-моделей
src/validator/                     проверка моделей
 tests/project_tests.rs             интеграционные тесты
```

## 4. Запуск приложения и CLI

### `src/main.rs`

Файл является точкой входа бинарного приложения.

- `main()` — вызывает `cli::cli_logic::cli_logic()`. Если функция возвращает ошибку, завершает процесс с кодом `1`. Сам текст ошибки печатается внутри логики CLI для соответствующих сценариев.

### `src/lib.rs`

Файл превращает проект в библиотеку, которую используют интеграционные тесты, и экспортирует модули:

- `api` — HTTP-запросы;
- `cli` — построение команд и их выполнение;
- `file_config` — файловые операции;
- `models` — структуры данных;
- `validator` — проверки;
- `errors` — общие ошибки;
- `generator` — генерация результатов.

Функций в этом файле нет.

### `src/cli/commands.rs`

Модуль собирает дерево команд с помощью `clap`.

- `build_cli() -> Command` — создаёт корневую команду `devinit`, задаёт версию `0.0.2`, автора, описание, обязательность subcommand и подключает все подкоманды.
- `build_add_cli() -> Command` — описывает `add <service>` с опциями `--port/-p` и `--version/-v`. Эти две опции разбираются, но пока не используются при запросе.
- `build_list_cli() -> Command` — описывает `list` с опциями `--type/-t` и `--page/-p`.
- `build_get_cli() -> Command` — описывает `get <service_url>`.
- `build_license_cli() -> Command` — описывает `license <license_type>`.
- `build_login_cli() -> Command` — описывает `login <login_url>`.
- `build_compile_cli() -> Command` — описывает `compile` без аргументов.
- `build_test_cli() -> Command` — описывает `test` с необязательным аргументом `tester`.

Зарегистрированные команды имеют разную степень готовности:

| Команда | Фактическое поведение |
| --- | --- |
| `add service` | Запрашивает рецепт у локального API и дописывает Compose/env-примеры |
| `get service_url` | Запрашивает список конфигураций и дописывает Compose-пример |
| `compile` | Читает, валидирует и генерирует файлы |
| `list` | Только печатает переданные фильтры |
| `login` | Только печатает URL; API login не вызывается |
| `license` | Зарегистрирована, отдельная обработка отсутствует |
| `test` | Ветка обработки пустая |

### `src/cli/cli_logic.rs`

Это основной маршрутизатор команд и публичная точка программного pipeline.

- `compile() -> Result<Vec<PathBuf>, DevinitError>` — запускает компиляцию стандартного файла `compile.yaml` в текущую директорию `.`. Это удобная обёртка над `compile_at`.
- `compile_at(input_path, output_dir) -> Result<Vec<PathBuf>, DevinitError>` — загружает `CompileSpec` через `load_compile`, вызывает `validate_compile`, преобразует список ошибок в `DevinitError::ValidationErrors`, а после успешной проверки вызывает `generator::generate`.
- `print_validation_errors(errors)` — приватно печатает в stderr все ошибки в формате «путь + сообщение».
- `cli_logic() -> Result<(), DevinitError>` — получает аргументы из `clap`, выбирает подкоманду и выполняет её.

Для `add` используются базовые URL:

```text
http://127.0.0.1:3000/services/
http://127.0.0.1:3000/configs/
```

`add` вызывает `add_recipe`, пишет рецепт в `compose.yaml.example` через `yaml_data` и переменные окружения в `env.example` через `env_file_config`. `get` вызывает `get_config` и добавляет полученные рецепты через `yaml_configs_data`. Ошибки этих операций печатаются, но внутри ветки команды не возвращаются как ошибка верхнего уровня.

## 5. Ошибки

### `src/errors.rs`

- `ValidationError` — небольшая структура с полями `path` и `message`. Представляет одну ошибку валидации и позволяет показать пользователю точное место проблемы.
- `GenerationError` — ошибка этапа генерации. Включает `Io` для создания/записи/переименования файлов и `Serialization` для ошибки сериализации YAML.
- `DevinitError` — общий enum приложения:
  - `HttpRequestError` — ошибка `reqwest`;
  - `JsonParseError` — ошибка десериализации JSON;
  - `FileIOError` — ошибка файловой системы;
  - `YamlParseError` — ошибка YAML;
  - `ValidationErrors` — список структурированных ошибок валидации;
  - `GenerationError` — ошибка генератора;
  - `InvalidInput`, `ServiceNotFound`, `ConfigurationError` — ошибки входных данных, сервиса и конфигурации;
  - `K8sValidationError`, `CiCdGenerationError`, `EnvConfigError`, `StorageError`, `ServiceGenerationError`, `Other` — расширения для будущих и отдельных подсистем. Не все варианты используются в текущем runtime-пути.

## 6. HTTP API

API-модули используют синхронный `reqwest::blocking::Client`. Каждый запрос проверяет HTTP-статус через `error_for_status()` и затем десериализует JSON.

### `src/api/add_recipe.rs`

- `add_recipe(recipe_name, base_url) -> Result<RecipeCompose, Box<dyn Error>>` — объединяет `base_url` и имя рецепта, выполняет GET с заголовком `Accept: application/json`, получает один `RecipeCompose` и возвращает его вызывающему коду.

### `src/api/get_config.rs`

- `get_config(config_url, config_name) -> Result<ConfigsList, Box<dyn Error>>` — выполняет GET по объединённому URL и получает объект `ConfigsList`, содержащий список рецептов.

### `src/api/login.rs`

- `login(company_url) -> Result<bool, Box<dyn Error>>` — выполняет POST, проверяет статус и ожидает JSON boolean. Функция реализована, но текущая CLI-ветка `login` её не вызывает.

## 7. Чтение и запись файлов

### `src/file_config/yaml_config.rs`

- `load_compile(path) -> Result<CompileSpec, DevinitError>` — проверяет существование входного файла, вызывает `ensure_file_not_empty`, читает строку UTF-8 и десериализует её через `serde_yaml`.
- `yaml_data(config_struct, path) -> io::Result<()>` — передаёт один `RecipeCompose` во внутреннюю функцию добавления рецептов.
- `yaml_configs_data(configs_list, path) -> io::Result<()>` — передаёт в неё все рецепты из `ConfigsList`.
- `append_recipes(recipes, path) -> io::Result<()>` — приватная общая реализация: пропускает пустой список, запрещает повторяющиеся Docker images, открывает файл в append-режиме, добавляет разделитель, удаляет поле `env` из Compose-части и сериализует каждый рецепт под ключом его имени.

Эти функции используются командами `add` и `get`, а не основным `compile`-генератором.

### `src/file_config/file_validator.rs`

- `ensure_file_not_empty(path)` — читает файл и возвращает `InvalidData`, если после удаления пробелов он пуст.
- `ensure_images_are_new(path, recipes)` — собирает images из существующего YAML и проверяет одновременно отсутствие повторов в файле и дублей внутри новой партии.
- `ensure_name_is_new(path, name)` — проверяет комментарии вида `# devinit config: <name>` в env-файле и запрещает повторное имя.
- `read_existing_images(path)` — приватно возвращает множество images существующих Compose-рецептов; для отсутствующего или пустого файла возвращает пустое множество.
- `read_existing_names(path)` — приватно читает строки env-файла и возвращает имена из специальных комментариев.

### `src/file_config/yaml_writer.rs`

- `write_yaml<T: Serialize>(value, path) -> Result<PathBuf, GenerationError>` — создаёт родительские директории, сериализует значение в YAML, пишет результат во временный файл с расширением `.tmp`, затем переименовывает его в целевой путь. Возвращает путь результата. Такой порядок уменьшает риск оставить частично записанный YAML.

### `src/file_config/env_config.rs`

- `env_file_config(recipe, path) -> io::Result<()>` — если `recipe.env` отсутствует или пуст, ничего не делает. Иначе проверяет уникальность имени, открывает файл в append-режиме, добавляет комментарий `# devinit config: ...` и записывает каждую env-строку.

## 8. Модели входных данных

### `src/models/compile_struct.rs`

- `CompileSpec` — корень `compile.yaml`: `services`, `kubernetes` и `cicd`.
- `CompileService` — untagged enum. Строка YAML разбирается как `BuiltIn(Services)`, объект — как `Custom(Box<RecipeCompose>)`.
- `CompileService::to_recipe()` — приводит оба варианта к единому `RecipeCompose`: встроенный сервис вызывает `Services::to_recipe`, пользовательский клонируется.

### `src/models/docker_compose_struct.rs`

- `RecipeCompose` — описание сервиса: `name`, `description`, `image`, `ports`, `environment`, `volumes`, `networks`, `depends_on`, `restart`, `command`, `files`, `env`, `notes`.
- `ConfigsList` — обёртка с полем `configs: Vec<RecipeCompose>` для API.
- `File` — вложенный файл с `path` и `content`.

Поля `files`, `env` и `notes` хранятся моделью, но в основном Docker Compose генераторе не выводятся. `env` используется отдельным `env_file_config` при импорте рецепта.

### `src/models/services_struct.rs`

`Services` содержит встроенные значения `postgresql`, `mysql`, `mongodb`, `mariadb`, `elasticsearch`, `prometheus`, `grafana`, `redis`, `rabbitmq`, `kafka`, `s3`, `ec2`.

- `Services::to_recipe()` — создаёт стандартный рецепт с именем, описанием, image, портами и политикой `unless-stopped`. Для S3 добавляет команду `server /data`.
- `services_selector(services)` — вызывает один из специализированных placeholder-validator-ов для части сервисов и печатает сообщение. Всегда возвращает `Ok(())`; результат валидатора сейчас не анализируется.

Значения в `to_recipe` захардкожены. `resources/services.yaml` является справочным файлом и в текущем коде не загружается.

### `src/models/env_struct.rs`

- `Env` — пустая структура-заготовка. Полей и методов нет.

### `src/models/cicd_struct.rs`

- `Pipeline` — CI/CD-конфигурация: `provider`, список `triggers` и список `jobs`.
- `CiProvider` — `Github` или `Gitlab`.
- `Trigger` — `Push { branches }`, `PullRequest { branches }` или `Manual`.
- `Job` — имя, runner, зависимости `needs` и шаги.
- `Runner` — `UbuntuLatest`, `WindowsLatest`, `MacosLatest`; значение по умолчанию — Ubuntu.
- `Step` — имя шага, действие `StepAction` и map переменных `env`.
- `StepAction` — `Checkout`, `Run { command }`, `DockerBuild { dockerfile, image }`, `DockerPush { image }`.

Serde использует snake_case для входных enum-значений, например `provider: github` и `action: docker_build`.

### `src/models/k8s_struct.rs`

Файл содержит типизированную модель Kubernetes Deployment и не содержит функций. Основные структуры:

- `Deployment` — `api_version`, `kind`, `metadata`, `spec`;
- `ObjectMeta` — имя, namespace, labels, annotations;
- `DeploymentSpec` — replicas, selector, template и параметры rollout;
- `DeploymentStrategy`, `RollingUpdate` — стратегия и ограничения обновления;
- `LabelSelector`, `PodTemplateSpec`, `PodSpec` — selector, шаблон Pod и настройки Pod;
- `Container`, `ContainerPort` — контейнер, image, команды, порты, env, probes, ресурсы и mounts;
- `EnvVar`, `EnvVarSource`, `KeySelector`, `ObjectFieldSelector` — переменные и источники значений;
- `EnvFromSource`, `ConfigMapEnvSource`, `SecretEnvSource` — массовый импорт env;
- `ResourceRequirements` — requests и limits;
- `VolumeMount`, `Volume` — подключение томов;
- `EmptyDirVolumeSource`, `ConfigMapVolumeSource`, `SecretVolumeSource`, `PersistentVolumeClaimVolumeSource`, `HostPathVolumeSource` — пять вариантов источника тома;
- `Probe`, `HttpGetAction`, `TcpSocketAction`, `ExecAction`, `GrpcAction` — проверки состояния;
- `SecurityContext`, `PodSecurityContext` — параметры безопасности;
- `LocalObjectReference` — ссылка на объект Kubernetes.

Большинство структур используют `camelCase` при сериализации (`apiVersion`, `imagePullPolicy`, `containerPort` и т. п.). Неуказанные optional-поля пропускаются благодаря `skip_serializing_if`.

## 9. Валидация

### `src/validator/compile_validator.rs`

- `validate_compile(spec) -> Result<(), Vec<ValidationError>>` — общий валидатор. Приводит каждый сервис к `RecipeCompose`, запускает проверку сервисов, Kubernetes и CI/CD, затем проверяет межсервисные ссылки и циклы. Возвращает все найденные ошибки сразу.
- `validate_cross_references(spec, errors)` — приватно проверяет дубли имён и существование каждого `depends_on`.
- `detect_cycles(spec, services, errors)` — запускает обход зависимостей для каждого сервиса.
- `visit_service(...) -> bool` — DFS-обход графа. Множество `visiting` обнаруживает обратное ребро, `visited` не даёт повторно обходить уже проверенные узлы.

### `src/validator/services_validator.rs`

- `validate_service(name, service, errors)` — проверяет имя, image, порты, environment, volumes, networks, `depends_on` и `files.path`.
- `validate_port(value, path, errors)` — приватно берёт часть после последнего `:`, разбирает её как `u16` и требует значение больше нуля.

Специализированные функции ниже публичны, но пока являются заглушками и всегда возвращают `true`:

- `validate_postgresql_service()`;
- `validate_mysql_service()`;
- `validate_mongodb_service()`;
- `validate_mariadb_service()`;
- `validate_elasticsearch_service()`;
- `validate_prometheus_service()`;
- `validate_grafana_service()`;
- `validate_redis_service()`;
- `validate_rabbitmq_service()`;
- `validate_kafka_service()`;
- `validate_s3_service()`;
- `validate_ec2_service()`.

### `src/validator/k8s_validator.rs`

- `validate_deployment(deployment, errors)` — проверяет имя Deployment, `apiVersion`, `kind`, положительное число replicas, наличие контейнеров, имя/image контейнеров, ненулевые container ports, корректность env, взаимоисключение `value` и `valueFrom`, имена volume mounts, mount paths, имена volume и наличие ровно одного источника volume.
- `validate_name(name, path, errors)` — приватно запрещает пустое имя.

Валидатор не проверяет полный Kubernetes schema contract: например, согласованность selector с labels и корректность всех числовых параметров остаются за пределами текущей реализации.

### `src/validator/cicd_validator.rs`

- `validate_pipeline(cicd, errors)` — проверяет непустые и уникальные имена job, непустые имена шагов, непустые команды `run` и существование всех имён в `needs`.
- `cicd_validator(cicd) -> io::Result<()>` — адаптер старого интерфейса: запускает основную проверку и при ошибках возвращает один `std::io::Error` с количеством проблем.

## 10. Генераторы

### `src/generator.rs`

- `generate(spec, output_dir) -> Result<Vec<PathBuf>, GenerationError>` — последовательно генерирует Compose, Kubernetes и CI/CD только для непустых/заданных частей `CompileSpec`. Возвращает список созданных путей.

### `src/generator/docker_compose_generator.rs`

- `generate_docker_compose(services, output_dir)` — преобразует все `CompileService` в рецепты, складывает их в `BTreeMap` по имени, строит Compose-модель и пишет `docker-compose.yaml`.
- `ComposeService::from_recipe(recipe)` — приватный конструктор, копирует поддерживаемые Compose-поля и преобразует `description` в label `com.devinit.description`.
- `DockerComposeFile` — внутренний корневой объект с map `services`.
- `ComposeService` — внутреннее представление. Сериализуются image, ports, environment, volumes, networks, depends_on, restart, command и labels. Поля `files`, `env`, `notes` не попадают в Compose.

### `src/generator/k8s_generator.rs`

- `generate_k8s(deployment, output_dir)` — формирует путь `<output_dir>/k8s/<metadata.name>-deployment.yaml`, передаёт Deployment в общий `write_yaml` и возвращает путь в однолючевом `Vec`.

### `src/generator/cicd_generator.rs`

- `generate_cicd(pipeline, output_dir)` — выбирает GitHub Actions или GitLab CI по `CiProvider`.
- `generate_github_actions(pipeline, output_dir)` — приватно строит `GithubWorkflow` и пишет `.github/workflows/devinit.yml`.
- `generate_gitlab_ci(pipeline, output_dir)` — приватно строит `GitlabPipeline` и пишет `.gitlab-ci.yml`.
- `GithubWorkflow::from_pipeline(pipeline)` — преобразует triggers и jobs во внутреннюю GitHub-модель.
- `github_trigger(branches)` — создаёт trigger с branches или пустой trigger, если список веток пуст.
- `runner_name(runner)` — преобразует runner в `ubuntu-latest`, `windows-latest` или `macos-latest`.
- `github_step(step)` — преобразует checkout в `actions/checkout@v4`, run в shell-команду, Docker build/push в соответствующие команды и переносит env.
- `GitlabPipeline::from_pipeline(pipeline)` — строит stages по именам job и map GitLab job-ов.
- `gitlab_command(step)` — преобразует run/build/push в script-команду; checkout в GitLab игнорируется и возвращает `None`.

GitHub результат содержит `name: DevInit`, `on` и `jobs`. В GitLab каждая job получает stage с тем же именем, а `needs` переносится напрямую.

## 11. Формат `compile.yaml`

Минимальная верхнеуровневая форма:

```yaml
services: []
kubernetes: null
cicd: null
```

### Встроенные сервисы

```yaml
services:
  - postgresql
  - redis
```

Поддерживаемые имена: `postgresql`, `mysql`, `mongodb`, `mariadb`, `elasticsearch`, `prometheus`, `grafana`, `redis`, `rabbitmq`, `kafka`, `s3`, `ec2`.

### Пользовательский сервис

```yaml
services:
  - name: backend
    description: API service
    image: ghcr.io/example/backend:1.0.0
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgres://postgres:5432/app
    depends_on:
      - postgresql
    restart: unless-stopped
```

Обязательны непустые `name` и `image`. Порты должны заканчиваться числовым значением больше нуля, environment — содержать `KEY=VALUE`, а зависимости должны ссылаться на существующие сервисы и не образовывать цикл.

### Kubernetes

```yaml
kubernetes:
  apiVersion: apps/v1
  kind: Deployment
  metadata:
    name: backend
  spec:
    replicas: 1
    selector:
      matchLabels: {app: backend}
    template:
      metadata:
        name: backend
        labels: {app: backend}
      spec:
        containers:
          - name: backend
            image: ghcr.io/example/backend:1.0.0
```

Минимально проверяются непустые `apiVersion`, `kind`, `metadata.name`, положительные `replicas`, один или более контейнеров и непустые имя/image контейнера.

### CI/CD

```yaml
cicd:
  provider: github
  triggers:
    - type: push
      branches: [main]
  jobs:
    - name: build
      runner: ubuntu-latest
      steps:
        - name: Checkout
          action: checkout
        - name: Build
          action: run
          command: cargo build --locked
```

Допустимы провайдеры `github` и `gitlab`, runner-ы `ubuntu-latest`, `windows-latest`, `macos-latest`, действия `checkout`, `run`, `docker_build`, `docker_push`.

## 12. Результаты генерации

Для такой конфигурации:

```yaml
services:
  - postgresql
kubernetes: null
cicd: null
```

создаётся:

```text
docker-compose.yaml
```

Если задан Kubernetes Deployment с именем `backend`, добавляется `k8s/backend-deployment.yaml`. Для GitHub добавляется `.github/workflows/devinit.yml`, для GitLab — `.gitlab-ci.yml`.

Повторная генерация заменяет файл через временный `.tmp` и не создаёт второй результат с тем же именем.

## 13. Тесты

### `tests/project_tests.rs`

Вспомогательные функции:

- `temp_paths(name)` — создаёт уникальные временные пути для `compile.yaml` и output;
- `valid_yaml()` — возвращает минимальный валидный YAML;
- `deployment()` — создаёт валидную тестовую Kubernetes-модель;
- `service(name, image, depends_on)` — создаёт базовый Compose-рецепт.

Тесты покрывают успешную компиляцию, отсутствие генерации при ошибке, malformed YAML, отсутствующий и пустой файл, повторную генерацию, одновременную генерацию трёх целей, структуру GitHub/GitLab, Compose labels, Kubernetes YAML, отсутствующую CI-зависимость, пустой image, неправильный порт, отсутствующую/циклическую зависимость и пустой image контейнера.

Запуск:

```bash
cargo test
```

## 14. Остальные файлы и зависимости

### `Cargo.toml`

Пакет использует edition 2024. Основные зависимости:

- `clap` — CLI;
- `serde`, `serde_json` — сериализация и JSON;
- `yaml_serde` под именем `serde_yaml` — YAML;
- `reqwest` — HTTP;
- `thiserror` — enum-ошибки;
- `ureq`, `inquire`, `dotenvy` — объявлены, но текущий просмотренный runtime-код их не использует.

### `resources/services.yaml`

Справочный YAML со списком встроенных сервисов, image, описанием, портами и командой S3. Сейчас не загружается программой: фактический источник значений — match в `Services::to_recipe()`.

### `README.md`

Краткое концептуальное описание целей проекта. Для точной схемы и текущего поведения следует использовать этот документ и `Instruction.md`.

### `Instruction.md`

Пользовательская инструкция по `compile.yaml`, команде `compile`, сервисам, Kubernetes и CI/CD. Содержит примеры и ограничения формата.

### `TODO.md`

План развития: dry-run/check, force/no-clobber, JSON-ошибки, Dockerfile и Compose-проверки, Kubernetes Service/probes/resources, профили окружений, registry login, cache и расширенная тестовая матрица.

### `compile.yaml`, `compile12.yaml`, `compile43.yaml`

Примеры входных данных:

- `compile.yaml` — PostgreSQL, Redis, пользовательский API и GitHub Actions;
- `compile12.yaml` — пример со встроенными сервисами;
- `compile43.yaml` — Compose, Kubernetes Deployment и GitLab CI с `needs`.

## 15. Известные ограничения

1. `list`, `license`, `test` и фактическая авторизация `login` пока не реализованы полностью.
2. `add` и `get` используют жёстко заданные URL `127.0.0.1:3000`.
3. Параметры `--port` и `--version` команды `add` не влияют на запрос.
4. Специализированные проверки встроенных сервисов возвращают `true` без проверки.
5. `resources/services.yaml` не является runtime-конфигурацией.
6. Поля `files` и `notes` не генерируются в Compose-файл.
7. Полная схема Kubernetes, согласованность labels/selectors, коллизии host ports и Dockerfile не проверяются.
8. Генератор перезаписывает существующие целевые файлы.
9. Значения env импортируемых рецептов записываются в `env.example`, но отдельная маскировка секретов не реализована.

Эти пункты отражены в `TODO.md` и должны учитываться при расширении проекта.
