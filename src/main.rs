use std::process::ExitCode;

fn main() -> ExitCode {
    let mut stdout = std::io::stdout();
    match usb_state_sender::run(
        std::env::args(),
        &mut stdout,
        &usb_state_sender::RuntimePaths::from_env(),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
