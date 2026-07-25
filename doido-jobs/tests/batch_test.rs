use doido_jobs::batch::Batch;

#[test]
fn batch_completes_when_all_members_finish() {
    let mut batch = Batch::new();
    batch.add("j1").add("j2").add("j3");
    assert_eq!(batch.progress(), (0, 3));
    assert!(!batch.is_complete());

    batch.complete("j1");
    batch.complete("j2");
    assert_eq!(batch.progress(), (2, 3));
    assert!(!batch.is_complete());

    batch.complete("j3");
    assert!(batch.is_complete());
    assert_eq!(batch.progress(), (3, 3));
}
