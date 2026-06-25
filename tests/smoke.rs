use std::process::Command;

#[test]
fn it_prints_tool_name() {
    let output = Command::new(env!("CARGO_BIN_EXE_usb-state-sender"))
        .output()
        .expect("binary should run");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "usbStateSender\n");
}
