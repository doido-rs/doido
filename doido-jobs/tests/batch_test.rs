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

#[test]
fn duplicate_add_counts_once() {
    let mut batch = Batch::new();
    batch.add("j1").add("j1");
    assert_eq!(batch.progress(), (0, 1));
}

#[test]
fn empty_batch_is_not_complete() {
    let batch = Batch::new();
    assert!(!batch.is_complete());
    assert_eq!(batch.progress(), (0, 0));
}
