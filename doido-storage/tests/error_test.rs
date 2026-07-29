use doido_storage::StorageError;

#[test]
fn display_messages_for_all_variants() {
    let cases = [
        StorageError::NotFound("blob".into()),
        StorageError::Io("disk".into()),
        StorageError::Backend("s3".into()),
        StorageError::Config("yaml".into()),
        StorageError::InvalidSignature("sig".into()),
        StorageError::Db("sql".into()),
    ];
    for err in cases {
        let msg = err.to_string();
        assert!(msg.contains("storage:"), "{msg}");
    }
}

#[test]
fn io_error_converts_to_storage_io() {
    let err = StorageError::from(std::io::Error::new(std::io::ErrorKind::NotFound, "missing"));
    assert!(matches!(err, StorageError::Io(_)));
}
