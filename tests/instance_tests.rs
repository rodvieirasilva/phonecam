use phonecam::instance::SingleInstanceLock;
use std::env;

#[test]
fn test_single_instance_lock_lifecycle() {
    let temp_dir = env::temp_dir();
    let lock_path = temp_dir.join(format!("phonecam_test_lock_{}.lock", std::process::id()));

    // 1. Acquire first lock
    let lock1 = SingleInstanceLock::acquire_at(lock_path.clone());
    assert!(lock1.is_ok(), "First lock acquisition must succeed");
    let lock1 = lock1.unwrap();

    // Verify lock file exists and contains our PID
    assert!(lock_path.exists());
    let pid_content = std::fs::read_to_string(&lock_path).unwrap();
    assert_eq!(pid_content.trim(), std::process::id().to_string());

    // 2. Attempt to acquire second lock on the same file -> MUST FAIL
    let lock2 = SingleInstanceLock::acquire_at(lock_path.clone());
    assert!(
        lock2.is_err(),
        "Second lock acquisition on same path must fail"
    );
    let err_msg = lock2.err().unwrap().to_string();
    assert!(
        err_msg.contains("already running"),
        "Error message should mention already running, got: {}",
        err_msg
    );

    // 3. Drop first lock
    drop(lock1);

    // 4. Acquire again after drop -> MUST SUCCEED
    let lock3 = SingleInstanceLock::acquire_at(lock_path.clone());
    assert!(lock3.is_ok(), "Lock acquisition after drop must succeed");
    drop(lock3);

    // Cleanup
    let _ = std::fs::remove_file(&lock_path);
}

#[test]
fn test_default_lock_path() {
    let path = SingleInstanceLock::default_lock_path();
    assert!(path.to_string_lossy().contains("phonecam"));
}
