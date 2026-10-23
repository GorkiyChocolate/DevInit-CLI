Текущий pipeline:

```text
compile.yaml
     ↓
   Parse
     ↓
 CompileSpec
     ↓
 Validate
     ↓
 Generate
     ↓
 Write files
```

А локальные сервисы пока:

```text
services.yaml
     ↓
 Deserialize
     ↓
 BTreeMap / HashMap
     ↓
 поиск нужного сервиса
```

Под это я бы составил такой roadmap.

### 1. Rust — обязательно

Не весь язык. Вот конкретно то, что понадобится DevInit:

```text
Rust
├── structs
├── enums
├── Option<T>
├── Result<T, E>
├── match
├── if let / let else
├── ownership
├── borrowing (&T / &mut T)
├── String / &str
├── Vec<T>
├── HashMap / BTreeMap
├── HashSet
├── iterators
├── closures — базово
├── modules
├── traits — базово
└── generics — базово
```

Особенно хорошо тебе надо понимать:

```rust
Option<String>
Result<CompileSpec, Error>
Vec<Service>
BTreeMap<String, Service>
```

Потому что примерно из таких типов будет состоять половина DevInit.

`Rc`, `Arc`, сложные lifetimes, macros, unsafe и продвинутый async Rust пока можешь вообще не трогать.

---

### 2. Serde + YAML

Это сейчас одна из **самых важных тем**.

Тебе надо самостоятельно уметь посмотреть на:

```yaml
services:
  backend:
    image: my-api:latest
    ports:
      - 8080

  postgres:
    image: postgres:17
    ports:
      - 5432
```

и написать под него Rust-модель:

```rust
struct CompileSpec {
    services: BTreeMap<String, Service>,
}

struct Service {
    image: String,
    ports: Vec<u16>,
}
```

Изучи:

```text
serde
├── Serialize
├── Deserialize
├── #[serde(...)]
├── default
├── rename
├── rename_all
├── flatten
├── skip_serializing_if
└── Option<T>
```

И работу YAML-библиотеки:

```text
YAML String
    ↓
deserialize
    ↓
Rust struct

Rust struct
    ↓
serialize
    ↓
YAML String
```

Это понадобится и для `compile.yaml`, и для `services.yaml`, и для генерации Kubernetes/Compose YAML.

---

### 3. Проектирование `CompileSpec`

Вот здесь уже начинается непосредственно DevInit.

Нужно изучить, как моделировать конфигурацию через типы.

Например:

```text
CompileSpec
├── project
├── services
├── networks
├── volumes
└── targets
```

А:

```text
Service
├── image
├── dockerfile
├── ports
├── environment
├── volumes
├── depends_on
└── command
```

Тут полезны понятия:

```text
Schema
Data Model
Domain Model
Required field
Optional field
Default value
Enum
Nested structure
```

Причём старайся использовать систему типов.

Вместо:

```rust
target: String
```

лучше:

```rust
enum Target {
    Compose,
    Kubernetes,
    Terraform,
}
```

Тогда невозможное значение вроде `"kuberenetes"` отсеется намного раньше.

---

### 4. Parsing

Тебе не нужно изучать написание парсеров или lexer'ов.

Поскольку YAML за тебя парсит библиотека, тебе достаточно понимать:

```text
File
 ↓
String
 ↓
YAML parser
 ↓
Deserialize
 ↓
CompileSpec
```

И главное — различать две ошибки.

Синтаксическая/структурная:

```yaml
ports:
  - hello
```

когда ожидается `u16`.

Это проблема parsing/deserialization.

А:

```yaml
depends_on:
  - database
```

когда `database` не существует — YAML абсолютно корректный.

Это уже проблема **validation**.

---

### 5. Validation — изучить хорошо

Это будет очень важная часть DevInit.

Тебе нужны два понятия:

```text
Structural validation
Semantic validation
```

Часть structural validation у тебя автоматически выполнит Serde.

А сам DevInit проверяет смысл.

Например:

```text
validate(spec)

├── validate_project()
├── validate_services()
├── validate_ports()
├── validate_dependencies()
├── validate_volumes()
└── validate_targets()
```

Нужно понимать принцип:

```text
parse != validate
```

`parse` отвечает:

> Могу ли я прочитать эту конфигурацию?

`validate`:

> Имеет ли эта конфигурация смысл?

---

### 6. Графы — только необходимый минимум

Для:

```yaml
backend:
  depends_on:
    - postgres
```

тебе понадобится понятие **directed graph**.

Например:

```text
backend ──→ postgres
worker  ──→ postgres
worker  ──→ redis
```

Изучи:

```text
Graph
Directed Graph
Node
Edge
DFS
Cycle Detection
Topological Sort
```

Особенно `DFS + Cycle Detection`.

Чтобы DevInit обнаруживал:

```text
A → B
↑   ↓
└── C
```

То есть:

```text
A → B → C → A
```

Не надо ради этого проходить полный курс алгоритмов.

---

### 7. Работа с файловой системой

Очень важно для CLI.

Rust:

```text
std::fs
std::path::Path
std::path::PathBuf
```

Понимать:

```text
relative path
absolute path
current directory
exists
is_file
is_dir
create_dir_all
read_to_string
write
```

Например:

```text
compile.yaml
     ↓
DevInit
     ↓
./devinit-output/
├── compose.yaml
├── kubernetes/
└── terraform/
```

Также разберись, что делать, если файл уже существует.

---

### 8. `services.yaml`

Это будет очень простая часть.

Тебе фактически нужны только:

```text
include_str!
serde
BTreeMap / HashMap
```

Pipeline:

```text
resources/services.yaml
          ↓
      include_str!
          ↓
    deserialize
          ↓
ServiceCatalog
          ↓
get("postgres")
          ↓
Service
```

То есть отдельной темы здесь почти нет.

---

### 9. Docker Compose — изучить хорошо

Это я бы сделал **первым генератором DevInit**.

Изучи Compose Specification на практическом уровне:

```text
services
├── image
├── build
├── command
├── ports
├── environment
├── env_file
├── volumes
├── networks
├── depends_on
├── restart
└── healthcheck

volumes
networks
```

И самое главное — сам руками напиши несколько `compose.yaml`.

Например:

```text
Go + PostgreSQL
Rust + PostgreSQL + Redis
Java + PostgreSQL
Backend + Frontend + PostgreSQL + Redis
```

Если ты сам не понимаешь Compose-файл, генератор Compose писать будет тяжело.

---

### 10. Генерация YAML

Не нужно генерировать YAML руками строками:

```rust
format!(
    "services:\n  {}:\n    image: {}\n",
    name,
    image
)
```

Лучше понять подход:

```text
CompileSpec
     ↓
Compose structures
     ↓
serde serialize
     ↓
compose.yaml
```

То есть у тебя условно:

```rust
struct ComposeFile {
    services: BTreeMap<String, ComposeService>,
    volumes: BTreeMap<String, ComposeVolume>,
}
```

Потом:

```text
CompileSpec
      ↓
generate_compose()
      ↓
ComposeFile
      ↓
serde YAML
      ↓
compose.yaml
```

Это намного надёжнее, чем вручную склеивать YAML.

---

### 11. Kubernetes — после Compose

Когда Compose заработает, изучай:

```text
Pod
Deployment
Service
ConfigMap
Secret
PersistentVolumeClaim
Ingress
Namespace
```

Плюс:

```text
metadata
spec
labels
selectors
containers
ports
env
volumes
resources
probes
```

Главное понимать отношения:

```text
Deployment
    ↓
Pod
    ↓
Container

Service
    ↓
selector
    ↓
Pod
```

Не лезь пока в Operators, CRD, Helm, Service Mesh и прочее.

---

### 12. Terraform — после Kubernetes

Для первой реализации:

```text
provider
resource
variable
output
module
state
dependency
```

И концепции:

```text
Declarative configuration
Desired state
Idempotency
Dependency graph
```

Terraform я бы вообще оставил третьим генератором.

Сначала:

```text
Compose → работает
```

потом:

```text
Compose + Kubernetes → работают
```

и только потом Terraform.

---

### 13. Clap и устройство CLI

Разобраться:

```text
Command
Subcommand
Argument
Option
Flag
```

Например:

```bash
devinit compile

devinit compile ./compile.yaml

devinit compile --target compose

devinit add postgres

devinit add redis --version 7
```

И научиться делать нормальные ошибки:

```text
Error: compile.yaml not found
Error: service "database" does not exist
Error: dependency cycle detected: api → worker → api
```

---

### 14. Error handling

Это тоже обязательно.

Изучи нормально:

```rust
Result<T, E>
?
map_err()
thiserror
anyhow
```

Не делай:

```rust
.unwrap()
.expect()
```

повсюду в CLI.

Пользовательская ошибка не должна приводить к:

```text
thread 'main' panicked at ...
```

DevInit должен отвечать понятно:

```text
Error: service "backend" references unknown service "postgresql"
```

---

### 15. Testing

Для DevInit это очень полезно.

Тебе нужны:

```text
Unit tests
Integration tests
```

Например, validator:

```text
valid config
→ OK

unknown dependency
→ Error

dependency cycle
→ Error

invalid port
→ Error
```

А generator:

```text
CompileSpec
    ↓
generate_compose()
    ↓
полученный ComposeFile
    ↓
проверяем services/ports/volumes
```

И это как раз поможет добиться твоей цели — **100% понятный и предсказуемый код**, вместо кучи архитектурных слоёв.

---

Если расставить всё по приоритету именно для тебя сейчас, я бы сделал так:

```text
1. Rust structs/enums/Option/Result
        ↓
2. Collections
   BTreeMap / HashMap / HashSet
        ↓
3. Serde + YAML
        ↓
4. CompileSpec
        ↓
5. Parsing
        ↓
6. Validation
        ↓
7. Graphs + cycle detection
        ↓
8. Filesystem
        ↓
9. Docker Compose
        ↓
10. YAML generation
        ↓
11. Error handling
        ↓
12. Testing
        ↓
───────────
DevInit Compose MVP готов
───────────
        ↓
13. Kubernetes
        ↓
14. Terraform
```
