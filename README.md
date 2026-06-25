# usbStateSender

This repository contains the Rust CLI for `usbStateSender`.
The current executable is a small placeholder that prints the tool name while the project is being built out.

## Setup

- Rust stable
- `vorbere` for task shortcuts

Build the project once after cloning:

```bash
vorbere run build
```

## Common Commands

- Run: `vorbere run run`
- Test: `vorbere run test`
- Format: `vorbere run fmt`
- Lint: `vorbere run clippy`
- CI-equivalent checks: `vorbere run ci`

## Project Structure

- `src/main.rs`: prints `usbStateSender`
- `tests/`: smoke test for the compiled binary
- `vorbere.yaml`: local development tasks

## Expected Output

```text
usbStateSender
```
