# Setup

## Linux

```
rustup target add i686-unknown-linux-gnu
```

## Windows

```
rustup target add i686-pc-windows-msvc
```

The meowtonin library requires the proper clang libraries, which can be installed like this:

```
winget LLVM.LLVM
```

# Compilation

```
# Linux
cargo build --release --target i686-unknown-linux-gnu
## Or with cargo-zigbuild
cargo zigbuild --release --target i686-unknown-linux-gnu

# Windows
cargo build --release --target i686-pc-windows-msvc
## Or with cargo-zigbuild
cargo zigbuild --release --target i686-pc-windows-msvc
```