use super::*;

fn spec(yaml: &str) -> CompileSpec {
    serde_yaml::from_str(yaml).unwrap()
}

#[test]
fn builds_valid_ir() {
    let config = spec(
        r#"
services:
  - name: api
    image: api:latest
  - name: redis
    image: redis:7
"#,
    );

    let ir = ResolvedCompileSpec::build(config).unwrap();

    assert_eq!(ir.services().len(), 2);
    assert!(ir.service("api").is_some());
    assert!(ir.service("redis").is_some());
}

#[test]
fn orders_dependencies() {
    let config = spec(
        r#"
services:
  - name: backend
    image: backend:latest
    depends_on:
      - redis

  - name: redis
    image: redis:7
"#,
    );

    let ir = ResolvedCompileSpec::build(config).unwrap();

    assert_eq!(
        ir.dependency_order(),
        &["redis".to_string(), "backend".to_string()]
    );
}

#[test]
fn returns_ordered_recipes() {
    let config = spec(
        r#"
services:
  - name: gateway
    image: gateway:latest
    depends_on:
      - backend

  - name: backend
    image: backend:latest
"#,
    );

    let ir = ResolvedCompileSpec::build(config).unwrap();

    let names: Vec<&str> = ir
        .ordered_services()
        .iter()
        .map(|recipe| recipe.name.as_str())
        .collect();

    assert_eq!(names, vec!["backend", "gateway"]);
}

#[test]
fn rejects_missing_dependency() {
    let config = spec(
        r#"
services:
  - name: backend
    image: backend:latest
    depends_on:
      - missing
"#,
    );

    assert!(ResolvedCompileSpec::build(config).is_err());
}

#[test]
fn rejects_circular_dependency() {
    let config = spec(
        r#"
services:
  - name: api
    image: api:latest
    depends_on:
      - worker

  - name: worker
    image: worker:latest
    depends_on:
      - api
"#,
    );

    assert!(ResolvedCompileSpec::build(config).is_err());
}

#[test]
fn preserves_original_specification() {
    let config = spec(
        r#"
services:
  - name: redis
    image: redis:7
"#,
    );

    let ir = ResolvedCompileSpec::build(config).unwrap();
    let original = ir.into_spec();

    assert_eq!(original.services.len(), 1);
    assert_eq!(original.services[0].to_recipe().image, "redis:7");
}
