use std::process::Command;
use tempfile::tempdir;

#[test]
fn it_writes_a_slot_update_to_the_matching_device() {
    let temp = tempdir().expect("tempdir should be created");
    let dev_root = temp.path().join("dev");
    let sys_root = temp.path().join("sys/class/hidraw");
    std::fs::create_dir_all(&dev_root).expect("dev root should be created");
    std::fs::create_dir_all(&sys_root).expect("sys root should be created");

    let device_name = "hidraw5";
    let device_path = dev_root.join(device_name);
    std::fs::write(&device_path, []).expect("device file should be created");

    let sys_device_dir = sys_root.join(device_name).join("device");
    std::fs::create_dir_all(&sys_device_dir).expect("sys device dir should be created");
    std::fs::write(sys_device_dir.join("idVendor"), "2e8a\n").expect("vendor id should be written");
    std::fs::write(sys_device_dir.join("idProduct"), "00a0\n")
        .expect("product id should be written");
    std::fs::write(sys_device_dir.join("product"), "Pico Inkpack Status\n")
        .expect("product string should be written");
    std::fs::write(sys_device_dir.join("interface"), "Status HID\n")
        .expect("interface string should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_usb-state-sender"))
        .env("USB_STATE_SENDER_DEV_ROOT", &dev_root)
        .env("USB_STATE_SENDER_SYS_HIDRAW_ROOT", &sys_root)
        .args(["--slot", "2", "IP: 192.0.2.55"])
        .output()
        .expect("binary should run");

    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("sent slot 2 update"),
        "stdout was {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let report = std::fs::read(device_path).expect("device report should be readable");
    assert_eq!(report.len(), 64);
    assert_eq!(report[0], 0x01);
    assert_eq!(report[1], 0x11);
    assert_eq!(report[2], 14);
    assert_eq!(&report[3..17], b"IP: 192.0.2.55");
}
