use rain_core::host_access::operate;
use serde_json::json;

#[tokio::test]
async fn explorer_handles_special_paths_and_reports_binary_and_missing_files() {
    let dir = std::env::temp_dir().join(format!("rain-files-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("code ' quoted # % + 日本語.txt");
    std::fs::write(&path, "old").unwrap();
    let listing = operate(None, json!({"action":"list","path":dir}))
        .await
        .unwrap();
    assert!(listing
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["path"] == path.to_string_lossy().as_ref()));
    operate(
        None,
        json!({"action":"write","path":path,"content":"remote-ready\n日本語"}),
    )
    .await
    .unwrap();
    let read = operate(None, json!({"action":"read","path":path}))
        .await
        .unwrap();
    assert_eq!(read["content"], "remote-ready\n日本語");
    std::fs::write(&path, [0, 255, 1]).unwrap();
    assert_eq!(
        operate(None, json!({"action":"read","path":path}))
            .await
            .unwrap()["is_binary"],
        true
    );
    assert!(
        operate(None, json!({"action":"read","path":dir.join("missing")}))
            .await
            .is_err()
    );
    assert!(operate(Some("not-enrolled"), json!({"action":"scan"}))
        .await
        .is_err());
    std::fs::remove_dir_all(dir).unwrap();
}
#[tokio::test]
async fn catalog_returns_executable_paths_without_running_them() {
    let catalog = operate(None, json!({"action":"scan"})).await.unwrap();
    assert!(catalog["executables"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["name"] == "sh"));
    assert!(catalog["shell"].as_str().unwrap().starts_with('/'));
    for tool in catalog["agents"].as_array().unwrap() {
        assert!(std::path::Path::new(tool["command"].as_str().unwrap()).is_file());
    }
}
