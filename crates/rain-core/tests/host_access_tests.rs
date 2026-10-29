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

#[tokio::test]
async fn file_transfer_preserves_binary_bytes_and_rejects_corruption() {
    let source=std::env::temp_dir().join(format!("rain-transfer-{}.bin",uuid::Uuid::new_v4()));
    let bytes=vec![0,1,254,255,0,10,13,240];std::fs::write(&source,&bytes).unwrap();
    let exported=operate(None,json!({"action":"export","path":source})).await.unwrap();
    let request=json!({"action":"import","name":exported["name"],"size_bytes":exported["size_bytes"],"sha256":exported["sha256"],"content_base64":exported["data_base64"]});
    let received=operate(None,request.clone()).await.unwrap();
    assert_eq!(received["sha256"],exported["sha256"]);
    let destination=std::path::PathBuf::from(received["path"].as_str().unwrap());
    assert_eq!(std::fs::read(&destination).unwrap(),bytes);
    let mut corrupt=request;corrupt["sha256"]=json!("0".repeat(64));
    assert!(operate(None,corrupt).await.unwrap_err().contains("SHA-256 mismatch"));
    std::fs::remove_file(source).unwrap();std::fs::remove_dir_all(destination.parent().unwrap()).unwrap();
}
