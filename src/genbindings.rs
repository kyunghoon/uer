use std::collections::HashMap;
use std::fs;
use std::path::Path;

const RUST_OUTPUT_PREFIX: &str = r#"
mod logger {
    use tracing::{field::{Field, Visit}, span, Event, Level, Metadata, Subscriber};
    use tracing_log::LogTracer;
    use tracing_subscriber::{layer::SubscriberExt, Layer, Registry};

    const ENABLE_THREAD_ID_LOGGING: bool = true;
    const MESSAGES_ONLY: bool = true;

    struct LogMessageVisitor<'a>(&'a Level);
    impl<'a> Visit for LogMessageVisitor<'a> {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            if MESSAGES_ONLY {
                if field.name() == "message" {
                    if ENABLE_THREAD_ID_LOGGING {
                        let thread_id_str = format!("{:?}", std::thread::current().id());
                        super::ue::internal_log(&format!("{} {:?}", &thread_id_str[8..], value), MyCustomLayer::to_lvl(self.0));
                    } else {
                        super::ue::internal_log(&format!("{:?}", value), MyCustomLayer::to_lvl(self.0));
                    }
                }
            } else {
                if ENABLE_THREAD_ID_LOGGING {
                    let thread_id_str = format!("{:?}", std::thread::current().id());
                    super::ue::internal_log(&format!("[{}] {} {:?}", field.name(), &thread_id_str[8..], value), MyCustomLayer::to_lvl(self.0));
                } else {
                    super::ue::internal_log(&format!("[{}] {:?}", field.name(), value), MyCustomLayer::to_lvl(self.0));
                }
            }
        }
    }

    struct MyCustomLayer;
    impl MyCustomLayer {
        fn to_lvl(level: &Level) -> u8 {
            if level == &tracing::Level::ERROR { 4 }
            else if level == &tracing::Level::WARN { 3 }
            else if level == &tracing::Level::INFO { 2 }
            else if level == &tracing::Level::DEBUG { 1 }
            else { 0 }
        }
    }
    impl<S: Subscriber> Layer<S> for MyCustomLayer {
        fn on_event(&self, event: &Event, _ctx: tracing_subscriber::layer::Context<'_, S>) {
            let mut visitor = LogMessageVisitor(event.metadata().level());
            event.record(&mut visitor);
        }

        fn on_new_span(&self, attrs: &span::Attributes, id: &span::Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
            super::ue::internal_log(&format!("New span: {:?} - {:?}", id, attrs), Self::to_lvl(attrs.metadata().level()));
        }

        fn enabled(&self, metadata: &Metadata, _ctx: tracing_subscriber::layer::Context<'_, S>) -> bool {
            metadata.level() <= &tracing::Level::INFO
        }
    }

    pub fn init() {
        LogTracer::init().expect("failed to set LogTracer");
        log::set_max_level(log::LevelFilter::Info);

        use tracing_subscriber::EnvFilter;
        let subscriber = Registry::default().with(MyCustomLayer);
        tracing::subscriber::set_global_default(subscriber).expect("failed to set default global tracer");
        let _  = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();
    }
}

extern "C" fn loaded(is_reload: bool) {
    logger::init();
    log::warn!("module was {}.", if is_reload { "reloaded" } else { "loaded" });
}

#[allow(non_snake_case)]
extern "C" fn invoke(
    method_id: u16,
    args: *const Argument,
    num_args: usize,
) -> Return {
    (GetUERustCApi().invoke)(method_id, args, num_args)
}

#[allow(non_snake_case)] #[repr(C)] pub struct UERustRsApi {
    pub loaded: extern "C" fn(is_reloaded: bool),
    pub invoke: extern "C" fn(method_id: u16, args: *const Argument, num_args: usize) -> Return,
}

#[allow(non_snake_case)] #[repr(C)] pub struct UERustCApi {
    pub internal_log: extern "C" fn (data: *const u8, i1: usize, lvl: u8),
    pub call_cpp: extern "C" fn (),
    pub invoke: extern "C" fn(method_id: u16, args: *const Argument, num_args: usize) -> Return,
}

#[allow(non_snake_case)] static CAPI: std::sync::OnceLock<UERustCApi> = std::sync::OnceLock::new();
#[allow(dead_code)] #[allow(non_snake_case)] fn GetUERustCApi() -> &'static UERustCApi { CAPI.get().expect("uerust capi not initialized") }
#[allow(non_snake_case)] #[unsafe(no_mangle)] pub extern "C" fn GetUERustRsApi_0(capi: UERustCApi) -> UERustRsApi {
    CAPI.get_or_init(|| capi);
    UERustRsApi {
        loaded,
        invoke,
    }
}

mod ue {
    pub fn internal_log(msg: &str, lvl: u8) {
        (super::GetUERustCApi().internal_log)(msg.as_ptr(), msg.len(), lvl);
    }
}
"#;

#[derive(Debug, Clone)]
struct TypeSpec {
    c_char: char,
    c_type: &'static str,
    rust_type: &'static str,
    size: usize,
    is_buf_like: bool,
}

impl TypeSpec {
    fn from_rust_type(ty: &str) -> Result<Self, String> {
        let ty = ty.trim();

        Ok(match ty {
            "()" => Self { c_char: 'v', c_type: "void", rust_type: "()", size: 0, is_buf_like: false },
            "bool" => Self { c_char: 'z', c_type: "bool", rust_type: "bool", size: 1, is_buf_like: false },
            "i8" => Self { c_char: 'b', c_type: "int8_t", rust_type: "i8", size: 1, is_buf_like: false },
            "u8" => Self { c_char: 'B', c_type: "uint8_t", rust_type: "u8", size: 1, is_buf_like: false },
            "i16" => Self { c_char: 's', c_type: "int16_t", rust_type: "i16", size: 2, is_buf_like: false },
            "u16" => Self { c_char: 'S', c_type: "uint16_t", rust_type: "u16", size: 2, is_buf_like: false },
            "i32" => Self { c_char: 'i', c_type: "int32_t", rust_type: "i32", size: 4, is_buf_like: false },
            "u32" => Self { c_char: 'I', c_type: "uint32_t", rust_type: "u32", size: 4, is_buf_like: false },
            "i64" => Self { c_char: 'j', c_type: "int64_t", rust_type: "i64", size: 8, is_buf_like: false },
            "u64" => Self { c_char: 'J', c_type: "uint64_t", rust_type: "u64", size: 8, is_buf_like: false },
            "f32" => Self { c_char: 'f', c_type: "float", rust_type: "f32", size: 4, is_buf_like: false },
            "f64" => Self { c_char: 'd', c_type: "double", rust_type: "f64", size: 8, is_buf_like: false },
            "&str" | "& str" | "str" => Self {
                c_char: 's',
                c_type: "uint8_t const*",
                rust_type: "&str",
                size: 0,
                is_buf_like: true,
            },
            "String" | "string" => Self {
                c_char: 'S',
                c_type: "uint8_t*",
                rust_type: "String",
                size: 0,
                is_buf_like: true,
            },
            _ => return Err(format!(
                "unsupported Rust type '{}' (supported: 'i32', 'f64', 'bool', '&str', 'String', '()', etc.)",
                ty
            )),
        })
    }
}

#[derive(Debug)]
struct FuncSpec {
    name: String,
    args: Vec<TypeSpec>,
    ret: TypeSpec,
    doc: String,
}

fn parse_api_toml(capi:& Path) -> Result<Vec<FuncSpec>, String> {
    let toml = fs::read_to_string(capi)
        .map_err(|e| format!("failed to read ./api.toml: {}", e))?;
    let value: toml::Value = toml::from_str(&toml)
        .map_err(|e| format!("invalid ./api.toml TOML: {}", e))?;

    let funcs = value["func"]
        .as_array()
        .ok_or("api.toml: missing [func] array")?
        .to_vec();

    let mut parsed = Vec::new();
    for (i, func) in funcs.iter().enumerate() {
        let table = func.as_table().ok_or(format!("api.toml: func[{}] must be a table", i))?;

        let name = table
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(format!("api.toml: func[{}] missing 'name'", i))?;
        let args = table
            .get("args")
            .and_then(|v| v.as_array())
            .ok_or(format!("api.toml: func[{}] missing 'args'", i))?;
        let ret = table
            .get("ret")
            .and_then(|v| v.as_str())
            .ok_or(format!("api.toml: func[{}] missing 'ret'", i))?;
        let doc = table.get("doc").and_then(|v| v.as_str()).unwrap_or("");

        let mut args_spec = Vec::new();
        for (j, arg) in args.iter().enumerate() {
            let ty = arg.as_str().ok_or(format!("api.toml: func[{}] arg[{}] must be string", i, j))?;
            args_spec.push(TypeSpec::from_rust_type(ty)?);
        }
        let ret_spec = TypeSpec::from_rust_type(ret)?;

        parsed.push(FuncSpec {
            name: name.to_string(),
            args: args_spec,
            ret: ret_spec,
            doc: doc.to_string(),
        });
    }

    Ok(parsed)
}

pub fn generate(capi: &Path, rsapi: &Path, rs_output_dir: &Path, uerust_plugin_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let funcs = parse_api_toml(capi)?;

    // --- Generate ./src/bindings.rs ---
    let mut rust = String::new();
    rust.push_str("// Auto-generated by gen-bindings.rs — do not edit\n");
    rust.push_str(RUST_OUTPUT_PREFIX);
    rust.push_str("\n");

    rust.push_str("use std::mem::ManuallyDrop;\n\n");

    rust.push_str("#[repr(C)]\n");
    rust.push_str("pub struct Buffer {\n");
    rust.push_str("    pub ptr: *const u8,\n");
    rust.push_str("    pub len: usize,\n");
    rust.push_str("}\n");

    rust.push_str("#[repr(u8)]\n");
    rust.push_str("pub enum ArgTag {\n");
    rust.push_str("    U8, I8, U16, I16, U32, I32, F32, F64, Buf,\n");
    rust.push_str("}\n");

    rust.push_str("#[repr(C)]\n");
    rust.push_str("pub union ArgType {\n");
    rust.push_str("    bool_val: bool,\n");
    rust.push_str("    u8_val: u8,\n");
    rust.push_str("    i8_val: i8,\n");
    rust.push_str("    u16_val: u16,\n");
    rust.push_str("    i16_val: i16,\n");
    rust.push_str("    u32_val: u32,\n");
    rust.push_str("    i32_val: i32,\n");
    rust.push_str("    f32_val: f32,\n");
    rust.push_str("    f64_val: f64,\n");
    rust.push_str("    buf_val: ManuallyDrop<Buffer>,\n");
    rust.push_str("}\n");

    rust.push_str("#[repr(C)]\n");
    rust.push_str("pub struct Argument {\n");
    rust.push_str("    pub tag: ArgTag,\n");
    rust.push_str("    pub value: ArgType,\n");
    rust.push_str("}\n");

    rust.push_str("#[repr(C)]\n");
    rust.push_str("pub struct Return {\n");
    rust.push_str("    pub is_some: bool,\n");
    rust.push_str("    pub data: Argument,\n");
    rust.push_str("}\n");

    rust.push_str("#[allow(non_camel_case_types)]\n");
    rust.push_str("#[repr(C)]\n");
    rust.push_str("enum MethodId {\n");
    for (i, func) in funcs.iter().enumerate() {
        let name = &func.name;
        rust.push_str(&format!("    {name} = {},\n", i + 1));
    }
    rust.push_str("}\n");
    rust.push_str("\n");

    for func in &funcs {
        let name = &func.name;

        rust.push_str(&format!("/// {} — {}\n", name, func.doc));
        rust.push_str(&format!("#[inline]\npub fn invoke_{}(", name));

        for (i, arg) in func.args.iter().enumerate() {
            if i == func.args.len() - 1 {
                rust.push_str(&format!("a{}: {}", i, arg.rust_type));
            } else {
                rust.push_str(&format!("a{}: {}, ", i, arg.rust_type));
            }
        }

        rust.push_str(") -> Return {\n");

        rust.push_str("    let args = [\n");
        for (i, arg) in func.args.iter().enumerate() {
            if arg.is_buf_like {
                rust.push_str(&format!("        Argument {{ tag: ArgTag::Buf, value: ArgType {{ buf_val: ManuallyDrop::new(Buffer {{ ptr: a{0}.as_ptr(), len: a{0}.len() }}) }} }},\n", i));
            } else {
                rust.push_str(&format!("        Argument {{ tag: ArgTag::{}, value: ArgType {{ {}_val: a{} }} }},\n", arg.rust_type.to_string().to_uppercase(), arg.rust_type, i));
            }
        }
        rust.push_str("    ];\n");
        rust.push_str(&format!("    (GetUERustCApi().invoke)(MethodId::{name} as u16, (&args).as_ptr(), args.len())\n"));

        rust.push_str("\n")
    }
    rust.push_str("}\n\n");

    fs::create_dir_all("./src")?;
    fs::write("./src/bindings.rs", rust)?;

    // --- Generate C++ bindings ---
    let mut h = String::new();
    h.push_str("// Auto-generated by gen-bindings.rs — do not edit\n");
    h.push_str("#pragma once\n\n");
    h.push_str("#include <cstdint>\n");
    h.push_str("#include <cstring>\n");
    h.push_str("#include <cstddef>\n\n");

    h.push_str("enum class MethodId {\n");
    for (i, func) in funcs.iter().enumerate() {
        let name = &func.name;
        h.push_str(&format!("    {name} = {},\n", i + 1));
    }
    h.push_str("};\n\n");

    h.push_str("struct Buffer {\n");
    h.push_str("    uint8_t const* ptr;\n");
    h.push_str("    size_t len;\n");
    h.push_str("};\n\n");
    
    h.push_str("enum class ArgTag : uint8_t {\n");
    h.push_str("    U8, I8, U16, I16, U32, I32, F32, F64, Buf\n");
    h.push_str("};\n\n");
    
    // Note: In C++, unions can hold objects with non-trivial constructors, 
    // but since we are using POD (Plain Old Data), this is safe.
    h.push_str("union ArgType {\n");
    h.push_str("    bool bool_val;\n");
    h.push_str("    uint8_t u8_val;\n");
    h.push_str("    int8_t i8_val;\n");
    h.push_str("    uint16_t u16_val;\n");
    h.push_str("    int16_t i16_val;\n");
    h.push_str("    uint32_t u32_val;\n");
    h.push_str("    int32_t i32_val;\n");
    h.push_str("    float f32_val;\n");
    h.push_str("    double f64_val;\n");
    h.push_str("    Buffer buf_val;\n");
    h.push_str("};\n\n");
    
    h.push_str("struct Argument {\n");
    h.push_str("    ArgTag tag;\n");
    h.push_str("    ArgType value;\n");
    h.push_str("};\n\n");

    h.push_str("struct Return {\n");
    h.push_str("    bool is_some;\n");
    h.push_str("    Argument value;\n");
    h.push_str("};\n\n");

    h.push_str("extern \"C\" Return __uerust_invoke_(uint16_t method_id, Argument const* args, size_t len);\n");

    let pub_dir = uerust_plugin_dir.join("Source").join("UERust").join("Public");
    fs::create_dir_all(&pub_dir)?;
    fs::write(pub_dir.join("bindings.h"), h)?;

    let mut cpp = String::new();
    cpp.push_str("// Auto-generated by gen-bindings.rs — do not edit\n");
    cpp.push_str("#include \"bindings.h\"\n#include <cstring>\n\n");

    cpp.push_str("extern \"C\" Return __uerust_invoke_(uint16_t method_id, Argument const* args, size_t len) {\n");

    for func in &funcs {
        cpp.push_str(&format!("    if (method_id == static_cast<uint16_t>(MethodId::{})) {{\n", func.name));
        for (i, arg) in func.args.iter().enumerate() {
            cpp.push_str(&format!("        // arg{}\n", i));
            cpp.push_str(&format!("        Argument const& arg{0} = args[{0}];\n", i));
            if arg.is_buf_like {
                cpp.push_str(&format!("        uint8_t const* arg{0}_ptr = arg{0}.value.buf_val.ptr;\n", i));
                cpp.push_str(&format!("        size_t arg{0}_len = arg{0}.value.buf_val.len;\n", i));
                cpp.push_str(&format!("        printf(\"Received buf: %.*s\\n\", (int)arg{0}_len, (char const*)arg{0}_ptr);\n", i));
            } else {
                cpp.push_str(&format!("        {0} const& arg{1}_b = arg{1}.value.{2}_val;\n", arg.c_type, i, arg.rust_type));
                cpp.push_str(&format!("        printf(\"Received {}\\n\");\n", arg.rust_type));
            }
            cpp.push_str("\n");
        }
    }
    cpp.push_str("        return Return { .is_some = false };\n");
    cpp.push_str("    }\n\n");

    cpp.push_str("    return Return { .is_some = false };\n");
    cpp.push_str("}\n");

    let priv_dir = uerust_plugin_dir.join("Source").join("UERust").join("Private");
    fs::create_dir_all(&priv_dir)?;
    fs::write(priv_dir.join("bindings.cpp"), cpp)?;

    println!("✅ Successfully generated bindings for {} functions", funcs.len());
    Ok(())
}
