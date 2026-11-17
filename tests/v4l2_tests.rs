use phonecam::v4l2::{is_device_present, is_module_loaded};

#[test]
fn test_is_device_present_nonexistent() {
    assert!(!is_device_present(99999));
}

#[test]
fn test_is_module_loaded() {
    // Should return a boolean without panicking
    let _ = is_module_loaded();
}
