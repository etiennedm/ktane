# Build & Run
Install Rust on your system, e.g. using rustup: https://rustup.rs/.

Install the `riscv32imac` target, and `espflash`:
```bash
rustup target add riscv32imac-unknown-none-elf
cargo install espflash
```

Then, build and run the project:
```bash
cargo run
```