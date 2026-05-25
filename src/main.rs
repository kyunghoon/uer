use std::{env, path::{Path, PathBuf}};
use xshell::{Shell, cmd};

type DynError = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, DynError>;

const MULTI_MODULES: bool = true;
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

pub enum Variant { Debug, Release }
impl Variant {
    fn as_str(&self) -> &str {
        match self {
            Variant::Debug => "Debug",
            Variant::Release => "Release",
        }
    }
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

fn copy_files_by_primary_stem(source: &Path, dest: &Path, target_stem: &str) -> Result<()> {
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

fn build_module(sh: Shell, ue_project_dir: &Path, plugin_params: &PluginParams, module_name: &str, module_params: &ModuleParams) -> Result<()> {
    sh.change_dir(module_name);
    sh.set_var("MODULE_NAME", module_name);

    let module_name = module_name;
    let variant = format!("{}", plugin_params.variant.as_str().to_lowercase());

    let mut flags = vec![];
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
        cmd!(sh, "install_name_tool -id @rpath/{lib_name}.dylib ../target/{variant}/{lib_name}.dylib").run()?;
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

fn print_help() {
    eprintln!("Tasks:
    build           compiles plugin
    android         compiles plugin for android");
}

fn main() {
    if let Err(e) = try_main() {
        eprintln!("{}", e);
        std::process::exit(-1);
    }
}

fn try_main() -> Result<()> {
    let ue_project_dir = std::path::PathBuf::from(&std::env::var("UE_PROJECT_DIR")
        .expect("UE_PROJECT_DIR is not set. Please define it in the .env file or pass it as an argument"));

    let plugin_params = PluginParams {
        name: "UERustPlugin",
        variant: Variant::Debug,
    };

    match env::args().nth(1).as_deref() {
        Some("build") => {
            #[cfg(target_os = "macos")]
            let target_os = TargetOS::Mac;
            #[cfg(target_os = "windows")]
            let target_os = TargetOS::Win64;

            let module_params = ModuleParams { abi: None, target_os, target_triple: None };
            let sh = Shell::new()?;
            sh.set_var("UE_PROJECT_DIR", &ue_project_dir);
            sh.set_var("PLUGIN_NAME", plugin_params.name);

            build_module(sh, &ue_project_dir, &plugin_params, "UERust", &module_params)?;
        },
        Some("android") => {
            let module_params = ModuleParams {
                abi: Some("arm64-v8a"),
                target_os: TargetOS::Android,
                target_triple: Some("aarch64-linux-android"),
            };

            let sh = Shell::new()?;
            sh.set_var("UE_PROJECT_DIR", &ue_project_dir);
            sh.set_var("PLUGIN_NAME", plugin_params.name);

            let ndk_root = sh.var("ANDROID_NDK_ROOT")?;
            let path = sh.var("PATH")?;
            sh.set_var("PATH", format!("{ndk_root}/toolchains/llvm/prebuilt/darwin-x86_64/bin:{path}"));
            sh.set_var("CC_aarch64_linux_android", "aarch64-linux-android21-clang");
            sh.set_var("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", "aarch64-linux-android21-clang");

            build_module(sh, &ue_project_dir, &plugin_params, "UERust", &module_params)?;
        }
        _ => print_help(),
    }
    Ok(())
}
