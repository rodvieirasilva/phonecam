use phonecam::adb::{DeviceStatus, parse_adb_devices, parse_battery_info, parse_scrcpy_cameras};

#[test]
fn test_parse_adb_devices_connected() {
    let output = r#"
* daemon not running; starting now at tcp:5037
* daemon started successfully
List of devices attached
RFC12345678	device usb:1-1 product:e1sxxx model:SM_S711B device:e1s transport_id:1
"#;
    let (serial, status) = parse_adb_devices(output);
    assert_eq!(serial, Some("RFC12345678".to_string()));
    assert_eq!(status, DeviceStatus::Connected);
}

#[test]
fn test_parse_adb_devices_unauthorized() {
    let output = r#"
List of devices attached
RFCW1234567	unauthorized usb:1-2 transport_id:2
"#;
    let (serial, status) = parse_adb_devices(output);
    assert_eq!(serial, Some("RFCW1234567".to_string()));
    assert_eq!(status, DeviceStatus::Unauthorized);
}

#[test]
fn test_parse_adb_devices_offline() {
    let output = r#"
List of devices attached
EMULATOR5554	offline transport_id:3
"#;
    let (serial, status) = parse_adb_devices(output);
    assert_eq!(serial, Some("EMULATOR5554".to_string()));
    assert_eq!(status, DeviceStatus::Offline);
}

#[test]
fn test_parse_adb_devices_empty() {
    let output = r#"
List of devices attached

"#;
    let (serial, status) = parse_adb_devices(output);
    assert_eq!(serial, None);
    assert_eq!(status, DeviceStatus::NotFound);
}

#[test]
fn test_parse_battery_info_charging() {
    let output = r#"
Current Battery Service state:
  AC powered: false
  USB powered: true
  Wireless powered: false
  Max charging current: 1500000
  Max charging voltage: 5000000
  Charge counter: 3500000
  status: 2
  health: 2
  present: true
  level: 84
  scale: 100
  voltage: 4150
  temperature: 285
  technology: Li-ion
"#;
    let (level, is_charging) = parse_battery_info(output);
    assert_eq!(level, Some(84));
    assert_eq!(is_charging, Some(true));
}

#[test]
fn test_parse_battery_info_full() {
    let output = r#"
Current Battery Service state:
  status: 5
  level: 100
"#;
    let (level, is_charging) = parse_battery_info(output);
    assert_eq!(level, Some(100));
    assert_eq!(is_charging, Some(true));
}

#[test]
fn test_parse_battery_info_discharging() {
    let output = r#"
Current Battery Service state:
  status: 3
  level: 42
"#;
    let (level, is_charging) = parse_battery_info(output);
    assert_eq!(level, Some(42));
    assert_eq!(is_charging, Some(false));
}

#[test]
fn test_parse_scrcpy_cameras() {
    let output = r#"
[server] INFO: Device: [samsung] samsung SM-S711B (Android 16)
[server] INFO: List of cameras:
    --camera-id=0    (back, 4080x3060, fps={15, 20, 24, 30}, zoom-range=[0.55, 8])
    --camera-id=1    (front, 3648x2736, fps={15, 20, 24, 30}, zoom-range=[1, 4])
    --camera-id=2    (back, 4000x3000, fps={15, 20, 24, 30}, zoom-range=[1, 8])
    --camera-id=3    (front, 3280x2268, fps={15, 20, 24, 30}, zoom-range=[1, 4])
"#;
    let cameras = parse_scrcpy_cameras(output);
    assert_eq!(cameras.len(), 4);

    assert_eq!(cameras[0].id, 0);
    assert_eq!(cameras[0].facing, "back");
    assert_eq!(cameras[0].resolution, "4080x3060");
    assert_eq!(cameras[0].fps_list, vec![15, 20, 24, 30]);
    assert_eq!(cameras[0].zoom_range, "0.55, 8");

    assert_eq!(cameras[1].id, 1);
    assert_eq!(cameras[1].facing, "front");
    assert_eq!(cameras[1].resolution, "3648x2736");
    assert_eq!(cameras[1].zoom_range, "1, 4");
}
