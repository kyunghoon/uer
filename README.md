# UERust: The Lightweight Rust-to-Unreal Engine Bridge

![Rust](https://img.shields.io/badge/rust-1.78+-orange.svg)
![Unreal Engine](https://img.shields.io/badge/unreal%20engine-5.0+-blue.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)

### Build, Bind, and Hot-Reload Rust code in Unreal Engine with minimal overhead.

UERust is a CLI-driven toolkit designed to bridge the gap between Rust and Unreal Engine (C++). It provides a robust, no-nonsense foundation for bi-directional FFI and dynamic library hot-reloading. Unlike opinionated frameworks, UERust acts as the "plumbing" layer, allowing you to focus on logic rather than fighting the build system.

## Features

- ✅ **Cross-platform builds**: Compile for Mac, Windows, and Android from any host system
- ✅ **Automatic bindings generation**: Generate C/Rust FFI bindings from JSON configuration
- ✅ **Unreal Engine plugin integration**: Auto-generate plugin boilerplate and integrate with UE subsystems
- ✅ **Flexible API definitions**: Define C-callable and Rust-callable functions with comprehensive type support
- ✅ **Seamless workflow**: Build, bind, and deploy Rust code to Unreal Engine projects
- ✅ **Type-safe FFI**: Comprehensive type mapping between C and Rust
- ✅ **Lightweight & minimal**: Designed to do *only one thing well*: bridge basic types and enable reliable plugin reload. No abstractions, no opinionated frameworks — just the bare essentials to get Rust code running and reloading in Unreal Engine. Complex functionality (actors, UObject wrappers, Blueprints, async, etc.) is intentionally left to be built *on top* of this foundation.

> 💡 **Philosophy**: UERust is a *toolkit*, not a framework. It avoids over-engineering and stays focused on the critical path: FFI, build automation, and hot-reload. Everything else — memory management strategies, UObject integrations, Blueprint bindings, async runtimes — is deliberately omitted so you can layer exactly what you need, without fighting against built-in assumptions.

## Quick Start

### Installation

```bash
# Install from source
cargo install --path .

# Or build and run directly
cargo build --release
./target/release/uer --help
```

### Basic Usage

1. **Install the tool**: `cargo install --path .` (ensure `~/.cargo/bin` is in your `PATH`)
2. **Navigate to your Unreal project**: `cd /path/to/your/project` (directory containing your `.uproject` file)
3. **Initialize the plugin**: Run `uer gen` — this creates `UERustPlugin/` and generates:
   - Plugin configuration (`UERustPlugin.uplugin`) and build files
   - C++ module implementation structure (`UERustModuleImpl.h` and concrete module class
   - `bindings.rs` contains the Rust bindings. You must export a `uerust_loaded` function in the module root to allow the module to initialize.
4. **Define your API**: Create `api.json` in your project root and add your function definitions
5. **Regenerate bindings**: Run `uer gen` again whenever you modify `api.json`
6. **Build and run**: Execute `uer build` to compile the Rust code into a dylib that Unreal Editor will automatically reload

## Project Structure

- `src/`: Rust CLI implementation
- `schema.json`: JSON Schema for API definitions
- `Cargo.toml`: Rust project configuration
- `Makefile`: Simple installation helper

> 💡 Note: `UERustPlugin/` is **generated automatically** by `uer gen` in your Unreal Engine project directory — it is not part of the UERust tool's source code.

## API Definition (`api.json`)

Define your Rust and C functions in `api.json`:

```json
{
  "cfunc": {
    "add": {
      "doc": "Add two integers",
      "ret": "i32",
      "args": [{"a": "i32"}, {"b": "i32"}]
    }
  },
  "rfunc": {
    "get_version": {
      "doc": "Get plugin version string",
      "ret": "&str",
      "args": []
    }
  }
}
```

Supported types: `bool`, `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `f32`, `f64`, `*f64`, `*mut f64`, `usize`, `isize`, `c_void`, `*c_void`, `*mut c_void`, `*u8`, `&str`, `String`

## Unreal Editor Integration

When using Rust for plugin development, Unreal Editor automatically detects changes to the compiled Rust dylib (e.g., after running `uer build`) and reloads it without requiring a manual editor restart. This enables rapid iteration — simply run `uer build` to update the dylib, and Unreal Editor will automatically detect and load the new version. Note that this automatic reload behavior applies to Rust dylibs but not to C++ modules, which require manual compilation and editor restart.


## Commands

### `uer gen`

Initialize the UERust plugin in your Unreal Engine project and generate C++ bindings. This command:
- Creates the `UERustPlugin/` directory structure in your project
- Generates `UERustPlugin/Source/UERust/Public/bindings.h` and `UERustPlugin/Source/UERust/Private/bindings.cpp`
- Sets up the plugin configuration (`UERustPlugin.uplugin`) and build files
- Creates `UERustModule` C++ implementation files if your project has a local module
- Updates the Unreal Engine project to recognize the new plugin

### `uer build`

Compile your Rust code into a dynamic library (dylib) and place it in the appropriate location for Unreal Engine to load.

- `--target-os`: Target OS (Mac, Win64, Android)
- `--variant`: Build variant (debug/release)
- `--module-name`: Module name (default: `UERust`)

> 💡 After `uer build` completes, Unreal Editor automatically detects the updated dylib and reloads it without requiring a restart.

### `uer android`

Build for Android with NDK support.

- `--android-ndk-root`: Path to Android NDK
- `--abi`: Android ABI (arm64-v8a, armeabi-v7a)
- `--target-triple`: Rust target triple

## Environment Variables

- `UE_PROJECT_DIR`: Path to your Unreal Engine project directory
- `ANDROID_NDK_ROOT`: Path to Android NDK (for Android builds)

## Requirements

- Rust 1.78+ (with `cargo`)
- Unreal Engine 5.0+
- For Android builds: Android NDK r21+
- For Windows builds: Visual Studio 2022+ (or MSVC toolchain)

## License

MIT License - see [LICENSE](LICENSE) file

## Contributing

Contributions are welcome! Please see our [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Acknowledgements

- Built with [Clap](https://crates.io/crates/clap) for CLI parsing
- Uses [XShell](https://crates.io/crates/xshell) for shell command execution
- Leverages [include_dir](https://crates.io/crates/include_dir) for plugin template embedding

---

**UERust** - Bridge the gap between Rust's performance and Unreal Engine's power.
