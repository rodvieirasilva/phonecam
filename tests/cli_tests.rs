use clap::Parser;
use phonecam::cli::Cli;

#[test]
fn test_cli_defaults() {
    let cli = Cli::try_parse_from(["phonecam"]).unwrap();
    assert!(!cli.gui);
    assert!(!cli.tui);
    assert!(!cli.start);
    assert!(!cli.detach);
    assert!(!cli.status);
    assert!(!cli.list_cameras);
    assert_eq!(cli.facing, "back");
    assert_eq!(cli.camera_id, None);
    assert_eq!(cli.res, "1920x1080");
    assert_eq!(cli.fps, 30);
    assert_eq!(cli.camera_ar, None);
    assert_eq!(cli.crop, None);
    assert!(!cli.mirror);
    assert!(!cli.flip_v);
    assert!(!cli.invert);
    assert!(!cli.preview);
    assert!(!cli.torch);
    assert_eq!(cli.video_nr, 10);
    assert_eq!(cli.port, 41789);
}

#[test]
fn test_cli_flags() {
    let args = [
        "phonecam",
        "--tui",
        "-f",
        "front",
        "--camera-id",
        "1",
        "-r",
        "1280x720",
        "--fps",
        "60",
        "--camera-ar",
        "16:9",
        "--crop",
        "1280:720:0:0",
        "-m",
        "--flip-v",
        "-i",
        "-p",
        "--torch",
        "--video-nr",
        "12",
        "--port",
        "45000",
    ];

    let cli = Cli::try_parse_from(args).unwrap();
    assert!(cli.tui);
    assert_eq!(cli.facing, "front");
    assert_eq!(cli.camera_id, Some(1));
    assert_eq!(cli.res, "1280x720");
    assert_eq!(cli.fps, 60);
    assert_eq!(cli.camera_ar, Some("16:9".to_string()));
    assert_eq!(cli.crop, Some("1280:720:0:0".to_string()));
    assert!(cli.mirror);
    assert!(cli.flip_v);
    assert!(cli.invert);
    assert!(cli.preview);
    assert!(cli.torch);
    assert_eq!(cli.video_nr, 12);
    assert_eq!(cli.port, 45000);
}

#[test]
fn test_cli_action_flags() {
    let cli_detach = Cli::try_parse_from(["phonecam", "-d"]).unwrap();
    assert!(cli_detach.detach);

    let cli_status = Cli::try_parse_from(["phonecam", "-s"]).unwrap();
    assert!(cli_status.status);

    let cli_list = Cli::try_parse_from(["phonecam", "--list-cameras"]).unwrap();
    assert!(cli_list.list_cameras);

    let cli_start = Cli::try_parse_from(["phonecam", "--start"]).unwrap();
    assert!(cli_start.start);

    let cli_wifi = Cli::try_parse_from(["phonecam", "--wifi", "192.168.1.100:5555"]).unwrap();
    assert_eq!(cli_wifi.wifi, Some("192.168.1.100:5555".to_string()));

    let cli_tcpip = Cli::try_parse_from(["phonecam", "--tcpip"]).unwrap();
    assert_eq!(cli_tcpip.tcpip, Some(None));
}
