# Инструкция по compile.yaml

`compile.yaml` - основной файл конфигурации DevInit. Команда `devinit compile` выполняет следующие шаги:

```text
compile.yaml
    -> чтение YAML
    -> CompileSpec
    -> валидация
    -> генерация файлов
```

Генерация выполняется только после успешной валидации.

## Запуск

Поместите `compile.yaml` в корень проекта и выполните:

```bash
devinit compile
```

Или запустите бинарный файл напрямую:

```bash
/path/to/devinit compile
```

Команда не принимает обязательных аргументов. Она всегда ищет файл `compile.yaml` в текущей рабочей директории.

## Общая структура

```yaml
services: []
kubernetes: null
cicd: null
```

Все три поля должны присутствовать:

- `services` - список сервисов для `docker-compose.yaml`;
- `kubernetes` - Kubernetes Deployment или `null`;
- `cicd` - CI/CD pipeline или `null`.

## Сервисы

### Встроенные сервисы

Встроенный сервис указывается коротким именем:

```yaml
services:
  - postgresql
  - redis
  - rabbitmq
```

Доступные имена:

```text
postgresql
mysql
mongodb
mariadb
elasticsearch
prometheus
grafana
redis
rabbitmq
kafka
s3
ec2
```

Для каждого встроенного сервиса DevInit знает Docker image, стандартные порты и описание. После успешной генерации создается `docker-compose.yaml`.

Пример:

```yaml
services:
  - postgresql
  - redis

kubernetes: null
cicd: null
```

Пример результата:

```yaml
services:
  postgresql:
    image: postgres:16
    ports:
      - 5432:5432
    restart: unless-stopped
    labels:
      com.devinit.description: PostgreSQL database
  redis:
    image: redis:7
    ports:
      - 6379:6379
    restart: unless-stopped
    labels:
      com.devinit.description: Redis in-memory data store
```

### Пользовательский сервис

Вместо встроенного имени можно передать полное описание сервиса:

```yaml
services:
  - postgresql
  - name: backend
    description: DevInit backend service
    image: ghcr.io/example/backend:latest
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgres://postgres:5432/app
    depends_on:
      - postgresql
    restart: unless-stopped
```

Поля пользовательского сервиса:

| Поле | Тип | Описание |
| --- | --- | --- |
| `name` | string | Уникальное имя сервиса |
| `description` | string или null | Описание сервиса |
| `image` | string | Docker image |
| `ports` | список строк или null | Порты в формате `host:container` |
| `environment` | список строк или null | Переменные в формате `KEY=VALUE` |
| `volumes` | список строк или null | Docker volumes |
| `networks` | список строк или null | Docker networks |
| `depends_on` | список строк или null | Зависимости от других сервисов |
| `restart` | string или null | Docker Compose restart policy |
| `command` | список строк или null | Команда контейнера |
| `files` | объект или null | Файл с `path` и `content` |
| `env` | список строк или null | Дополнительные env-значения |
| `notes` | список строк или null | Заметки |

Для каждого сервиса обязательно указывать непустые `name` и `image`.

## Kubernetes

Если Kubernetes не нужен, укажите:

```yaml
kubernetes: null
```

Если нужен Deployment, минимальная конфигурация выглядит так:

```yaml
kubernetes:
  apiVersion: apps/v1
  kind: Deployment
  metadata:
    name: backend
  spec:
    replicas: 1
    selector:
      matchLabels:
        app: backend
    template:
      metadata:
        name: backend
        labels:
          app: backend
      spec:
        containers:
          - name: backend
            image: ghcr.io/example/backend:latest
            ports:
              - containerPort: 8080
```

Основные правила:

- `apiVersion`, `kind` и `metadata.name` не должны быть пустыми;
- `replicas` должно быть больше нуля;
- `spec.selector` обязателен;
- `spec.template` обязателен;
- `containers` должен содержать хотя бы один контейнер;
- у контейнера должны быть непустые `name` и `image`;
- `containerPort` должен быть больше нуля.

После успешной генерации создается:

```text
k8s/<metadata.name>-deployment.yaml
```

## CI/CD

Если CI/CD не нужен:

```yaml
cicd: null
```

### GitHub Actions

```yaml
cicd:
  provider: github
  triggers:
    - type: push
      branches:
        - main
    - type: pull_request
      branches:
        - main
    - type: manual
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

Доступные runner-ы:

```text
ubuntu-latest
windows-latest
macos-latest
```

Доступные действия:

```yaml
# Получить исходный код
- name: Checkout
  action: checkout

# Выполнить команду
- name: Test
  action: run
  command: cargo test

# Собрать Docker image
- name: Build image
  action: docker_build
  dockerfile: Dockerfile
  image: example/backend:latest

# Отправить Docker image
- name: Push image
  action: docker_push
  image: example/backend:latest
```

Файл результата:

```text
.github/workflows/devinit.yml
```

### GitLab CI

```yaml
cicd:
  provider: gitlab
  jobs:
    - name: build
      runner: ubuntu-latest
      steps:
        - name: Build
          action: run
          command: cargo build --locked
    - name: test
      runner: ubuntu-latest
      needs:
        - build
      steps:
        - name: Test
          action: run
          command: cargo test
```

Файл результата:

```text
.gitlab-ci.yml
```

`needs` должен ссылаться на существующее имя job.

## Полный пример

```yaml
services:
  - postgresql
  - redis

kubernetes:
  apiVersion: apps/v1
  kind: Deployment
  metadata:
    name: backend
  spec:
    replicas: 1
    selector:
      matchLabels:
        app: backend
    template:
      metadata:
        name: backend
        labels:
          app: backend
      spec:
        containers:
          - name: backend
            image: ghcr.io/example/backend:latest
            ports:
              - containerPort: 8080

cicd:
  provider: github
  triggers:
    - type: push
      branches:
        - main
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

Ожидаемые файлы:

```text
docker-compose.yaml
k8s/backend-deployment.yaml
.github/workflows/devinit.yml
```

## Ошибки валидации

DevInit выводит все найденные ошибки и не создает файлы, если конфигурация невалидна.

Пример:

```text
Configuration validation failed:

services.backend.image
  cannot be empty

kubernetes.spec.replicas
  must be greater than 0
```

Исправьте ошибки в `compile.yaml` и запустите команду повторно.

## Важно

DevInit только генерирует файлы. Команда `devinit compile` не выполняет:

- `docker compose up`;
- `kubectl apply`;
- запуск CI/CD pipeline;
- публикацию Docker images;
- сетевые deployment-команды.
