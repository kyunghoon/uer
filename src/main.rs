mod genbindings;

use std::{path::{Path, PathBuf}};
use clap::{Parser, Subcommand};
use xshell::{Shell, cmd};

const MULTI_MODULES: bool = true;

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetOS { Mac, Android, Win64 }
impl TargetOS {
    fn as_str(&self) -> &str {
        match self {
            TargetOS::Mac => "Mac",
            TargetOS::Android => "Android",
            TargetOS::Win64 => "Win64",
        }
    }
    fn lib_name(&self, name: &str) -> String {
        match self {
            TargetOS::Mac | TargetOS::Android => format!("lib{name}"),
            _ => name.to_owned(),
        }
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant { Debug, Release }
impl Variant {
    fn as_str(&self) -> &str {
        match self {
            Variant::Debug => "Debug",
            Variant::Release => "Release",
        }
    }
}

pub fn get_project_name() -> Result<Option<String>, Box<dyn std::error::Error>> {
    let current_dir = std::env::current_dir()?;
    if let Ok(entries) = std::fs::read_dir(current_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            
            // 3. Check if the file has the .uproject extension
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("uproject") {
                // 4. Extract the file stem (e.g., "Gamebit" from "Gamebit.uproject")
                if let Some(project_name) = path.file_stem().and_then(|s| s.to_str()) {
                    return Ok(Some(project_name.to_owned()));
                }
            }
        }
    }
    Ok(None)
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Unreal Engine Rust Plugin Builder", long_about = None)]
struct Cli {
    /// Path to the Unreal Engine project directory (e.g., /path/to/MyProject).
    /// If not provided, auto-detected from nearest .uproject file (upward search),
    /// or falls back to $UE_PROJECT_DIR env var.
    #[arg(long)]
    ue_project_dir: Option<PathBuf>,

    /// Name of the plugin (e.g., UERustPlugin)
    #[arg(long, default_value = "UERustPlugin")]
    plugin_name: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Build for the host platform (Mac, Win64, etc.)
    Build {
        /// Target OS (defaults to host OS)
        #[arg(long, value_enum)]
        target_os: Option<TargetOS>,

        /// Target triple for cross-compilation (e.g., x86_64-apple-darwin)
        #[arg(long)]
        target_triple: Option<String>,

        /// Build variant: debug or release
        #[arg(long, value_enum, default_value = "debug")]
        variant: Variant,

        /// Module name (default: UERust)
        #[arg(long, default_value = "UERust")]
        module_name: String,
    },

    /// Build for Android (requires NDK)
    Android {
        /// Android NDK root directory
        #[arg(long)]
        android_ndk_root: Option<PathBuf>,

        /// ABI (e.g., arm64-v8a, armeabi-v7a)
        #[arg(long, default_value = "arm64-v8a")]
        abi: String,

        /// Target triple (e.g., aarch64-linux-android)
        #[arg(long, default_value = "aarch64-linux-android")]
        target_triple: String,

        /// Build variant: debug or release
        #[arg(long, value_enum, default_value = "debug")]
        variant: Variant,

        /// Module name (default: UERust)
        #[arg(long, default_value = "UERust")]
        module_name: String,
    },
    Bindgen,
}

struct PluginParams<'a> {
    name: &'a str,
    variant: Variant,
}

struct ModuleParams<'a> {
    abi: Option<&'a str>,
    target_os: TargetOS,
    target_triple: Option<&'a str>,
}

fn copy_files_by_primary_stem(source: &Path, dest: &Path, target_stem: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !source.is_dir() { 
        return Err(format!("Source not found: {}", source.display()).into()); 
    }
    std::fs::create_dir_all(dest)?;

    for entry in std::fs::read_dir(source)? {
        let path = entry?.path();
        if path.is_file() {
            // Get the full filename (e.g., "data.dll.lib")
            if let Some(filename_os) = path.file_name().and_then(|s| s.to_str()) {
                // Split by '.' and take the first part ("data")
                let primary_stem = filename_os.split('.').next().unwrap_or("");

                if primary_stem == target_stem {
                    let dest_file = dest.join(filename_os);
                    std::fs::copy(&path, dest_file)?;
                }
            }
        }
    }
    Ok(())
}

fn build_module(
    sh: Shell,
    ue_project_dir: &Path,
    plugin_params: &PluginParams,
    module_name: &str,
    module_params: &ModuleParams,
) -> Result<(), Box<dyn std::error::Error>> {
    // sh.change_dir(module_name);
    sh.set_var("MODULE_NAME", module_name);

    let module_name = module_name;
    let variant = format!("{}", plugin_params.variant.as_str().to_lowercase());

    let mut flags = vec![];
    // flags.push("--lib=UERust".to_string());

    if let Some(target_triple) = module_params.target_triple {
        flags.push(format!("--target={target_triple}"));
    }
    if !matches!(plugin_params.variant, Variant::Debug) {
        flags.push("--release".to_owned());
    }
    let flags_str = flags.join(" ");
    if flags_str.is_empty() { cmd!(sh, "cargo build") } else { cmd!(sh, "cargo build {flags_str}") }.run()?;

    let lib_name = module_params.target_os.lib_name(module_name);
    if matches!(module_params.target_os, TargetOS::Mac) {
        cmd!(sh, "install_name_tool -id @rpath/{lib_name}.dylib ./target/{variant}/{lib_name}.dylib").run()?;
    }

    let mut dst_dir = ue_project_dir
        .join("Plugins")
        .join(plugin_params.name)
        .join("Source");
    if MULTI_MODULES { dst_dir = dst_dir.join(module_name); }
    dst_dir = dst_dir.join(module_params.target_os.as_str());
    if let Some(abi) = module_params.abi { dst_dir = dst_dir.join(abi); }

    let mut src_dir = PathBuf::new().join("target");
    if let Some(target_triple) = module_params.target_triple { src_dir = src_dir.join(target_triple); }

    std::fs::create_dir_all(&dst_dir)?;
    copy_files_by_primary_stem(&src_dir.join(variant), &dst_dir, &lib_name)?;

    Ok(())
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Resolve ue_project_dir: cli arg > env > auto-discover
    let ue_project_dir = {
        if let Some(p) = &cli.ue_project_dir {
            p.clone()
        } else if let Ok(val) = std::env::var("UE_PROJECT_DIR") {
            PathBuf::from(val)
        } else {
            // Search upward for any *.uproject file
            let mut dir = std::env::current_dir()?;
            let ue_dir = 'outer: loop {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.filter_map(Result::ok) {
                        let path = entry.path();
                        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("uproject") {
                            break 'outer Some(dir.clone());
                        }
                    }
                }
                if !dir.pop() {
                    break 'outer None;
                }
            };
            let Some(dir) = ue_dir else {
                return Err("No .uproject file found in current or parent directories, and UE_PROJECT_DIR not set".into());
            };
            dir
        }
    };

    match &cli.command {
        Commands::Build { variant, target_os, target_triple, module_name } => {
            let plugin_params = PluginParams {
                name: &cli.plugin_name,
                variant: *variant,
            };
            // Auto-detect target_os if not specified
            let target_os = target_os.clone().unwrap_or_else(|| {
                if cfg!(target_os = "macos") { TargetOS::Mac }
                else if cfg!(target_os = "windows") { TargetOS::Win64 }
                else { panic!("Unsupported host OS") }
            });

            let module_params = ModuleParams {
                abi: None,
                target_os,
                target_triple: target_triple.as_deref(),
            };

            let sh = Shell::new()?;
            sh.set_var("UE_PROJECT_DIR", &ue_project_dir);
            sh.set_var("PLUGIN_NAME", &cli.plugin_name);

            build_module(sh, &ue_project_dir, &plugin_params, module_name, &module_params)?;
        }
        Commands::Android { android_ndk_root, abi, target_triple, module_name, variant } => {
            let plugin_params = PluginParams { name: &cli.plugin_name, variant: *variant };

            let module_params = ModuleParams {
                abi: Some(abi.as_str()),
                target_os: TargetOS::Android,
                target_triple: Some(target_triple.as_str()),
            };

            let sh = Shell::new()?;
            sh.set_var("UE_PROJECT_DIR", &ue_project_dir);
            sh.set_var("PLUGIN_NAME", &cli.plugin_name);

            let ndk_root = {
                if let Some(p) = &android_ndk_root {
                    p.clone()
                } else if let Ok(val) = std::env::var("ANDROID_NDK_ROOT") {
                    PathBuf::from(val)
                } else {
                    return Err("No .uproject file found in current or parent directories, and UE_PROJECT_DIR not set".into());
                }
            };
            let path = sh.var("PATH")?;
            sh.set_var("PATH", format!("{}/toolchains/llvm/prebuilt/darwin-x86_64/bin:{}", ndk_root.display(), path));
            sh.set_var("CC_aarch64_linux_android", "aarch64-linux-android21-clang");
            sh.set_var("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", "aarch64-linux-android21-clang");

            build_module(sh, &ue_project_dir, &plugin_params, module_name, &module_params)?;
        }
        Commands::Bindgen => {
            genbindings::generate(
                &ue_project_dir.join("api.toml"),
                &ue_project_dir.join("src"),
                get_project_name()?.map(|name| ue_project_dir.join("Source").join(name)),
            )?;
        }
    }

    Ok(())
}
