use serde::Deserialize;
use std::collections::HashMap;
use std::{fs, path::PathBuf};
use std::path::Path;
use include_dir::{include_dir, Dir};

static ASSETS: Dir = include_dir!("UERustPlugin");

#[derive(Debug, Clone)]
struct TypeSpec {
    c_type: &'static str,
    rust_type: &'static str,
    rust_macro_type: &'static str,
    is_buf_like: bool,
    tag: &'static str,
}

impl TypeSpec {
    fn from_rust_type(ty: &str) -> Result<Self, String> {
        let ty = ty.trim();

        Ok(match ty {
            "()" => Self { c_type: "void", rust_type: "()", rust_macro_type: "()", is_buf_like: false, tag: "Unit" },
            "bool" => Self { c_type: "bool", rust_type: "bool", rust_macro_type: "bool", is_buf_like: false, tag: "Bool" },
            "i8" => Self { c_type: "int8_t", rust_type: "i8", rust_macro_type: "u8", is_buf_like: false, tag: "I8" },
            "u8" => Self { c_type: "uint8_t", rust_type: "u8", rust_macro_type: "u8", is_buf_like: false, tag: "U8" },
            "i16" => Self { c_type: "int16_t", rust_type: "i16", rust_macro_type: "i16", is_buf_like: false, tag: "I16" },
            "u16" => Self { c_type: "uint16_t", rust_type: "u16", rust_macro_type: "u16", is_buf_like: false, tag: "U16" },
            "i32" => Self { c_type: "int32_t", rust_type: "i32", rust_macro_type: "i32", is_buf_like: false, tag: "I32" },
            "u32" => Self { c_type: "uint32_t", rust_type: "u32", rust_macro_type: "u32", is_buf_like: false, tag: "U32" },
            "i64" => Self { c_type: "int64_t", rust_type: "i64", rust_macro_type: "i64", is_buf_like: false, tag: "I64" },
            "u64" => Self { c_type: "uint64_t", rust_type: "u64", rust_macro_type: "u64", is_buf_like: false, tag: "U64" },
            "f32" => Self { c_type: "float", rust_type: "f32", rust_macro_type: "f32", is_buf_like: false, tag: "F32" },
            "f64" => Self { c_type: "double", rust_type: "f64", rust_macro_type: "f64", is_buf_like: false, tag: "F64" },
            "*f64" => Self { c_type: "double const*", rust_type: "*const f64", rust_macro_type: "&f64", is_buf_like: false, tag: "F64CPtr" },
            "*mut f64" => Self { c_type: "double*", rust_type: "*mut f64", rust_macro_type: "&mut f64", is_buf_like: false, tag: "F64Ptr" },
            "usize" => Self { c_type: "uintptr_t", rust_type: "usize", rust_macro_type: "usize", is_buf_like: false, tag: "USize" },
            "isize" => Self { c_type: "size_t", rust_type: "isize", rust_macro_type: "isize", is_buf_like: false, tag: "ISize" },
            "c_void" => Self { c_type: "void*", rust_type: "*mut std::ffi::c_void", rust_macro_type: "Box<_>", is_buf_like: false, tag: "Void", },
            "*c_void" => Self { c_type: "void const*", rust_type: "*const std::ffi::c_void", rust_macro_type: "&Box<_>", is_buf_like: false, tag: "CPtr", },
            "*mut c_void" => Self { c_type: "void*", rust_type: "*mut std::ffi::c_void", rust_macro_type: "&mut Box<_>", is_buf_like: false, tag: "Ptr", },
            "&str" | "& str" | "str" => Self { c_type: "uint8_t const*", rust_type: "&str", rust_macro_type: "&str", is_buf_like: true, tag: "Buf", },
            "String" | "string" => Self { c_type: "uint8_t*", rust_type: "String", rust_macro_type: "String", is_buf_like: true, tag: "String", },
            _ => return Err(format!(
                "unsupported Rust type '{}' (supported: 'i32', 'f64', 'bool', '&str', 'String', '()', etc.)",
                ty
            )),
        })
    }
}

#[derive(Debug, Deserialize)]
struct Arg {
    #[serde(flatten)]
    inner: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct FuncEntry {
    #[serde(default)]
    doc: String,
    #[serde(default)]
    ret: Option<String>,
    #[serde(default)]
    args: Vec<Arg>,
}

#[derive(Debug, Deserialize)]
struct ApiSchema {
    cfunc: HashMap<String, FuncEntry>,
    rfunc: HashMap<String, FuncEntry>,
}

#[derive(Debug)]
struct FuncSpec {
    name: String,
    args: Vec<(String, TypeSpec)>, // Assuming TypeSpec::from_rust_type exists
    ret: TypeSpec,
    doc: String,
}

fn parse_api_json(path: &Path) -> Result<(Vec<FuncSpec>, Vec<FuncSpec>), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file `{:?}`: {}", path, e))?;
    let data: ApiSchema = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let process_map = |map: HashMap<String, FuncEntry>| -> Result<Vec<FuncSpec>, String> {
        map.into_iter().map(|(name, entry)| {
            let mut args_spec = Vec::new();
            for arg in entry.args {
                for (key, ty) in arg.inner {
                    args_spec.push((key, TypeSpec::from_rust_type(&ty)?));
                }
            }
            
            Ok(FuncSpec {
                name,
                args: args_spec,
                ret: TypeSpec::from_rust_type(&entry.ret.unwrap_or_else(|| "()".to_string()))?,
                doc: entry.doc,
            })
        }).collect()
    };

    let mut cfuncs = process_map(data.cfunc)?;
    let mut rfuncs = process_map(data.rfunc)?;

    cfuncs.push(FuncSpec {
        name: "uerust_log".to_owned(),
        args: vec![
            ("a0".to_owned(), TypeSpec::from_rust_type("&str")?),
            ("a1".to_owned(), TypeSpec::from_rust_type("u8")?),
        ],
        ret: TypeSpec::from_rust_type("()")?,
        doc: "".to_owned(),
    });

    rfuncs.push(FuncSpec {
        name: "uerust_loaded".to_owned(),
        args: vec![("a0".to_owned(), TypeSpec::from_rust_type("bool")?)],
        ret: TypeSpec::from_rust_type("()")?,
        doc: "".to_owned(),
    });

    cfuncs.sort_by(|a, b| a.name.cmp(&b.name));
    rfuncs.sort_by(|a, b| a.name.cmp(&b.name));

    Ok((cfuncs, rfuncs))
}

pub fn generate(api: &Path, rs_output_dir: &Path, project_module_dir: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let api_dst_path = std::env::current_dir()?.join("api.json");
    if !api_dst_path.exists() {
        let mut api = String::new();
        api.push_str(r#"{
    "$schema": "https://raw.githubusercontent.com/kyunghoon/uer/refs/heads/main/schema.json",
    "cfunc": {},
    "rfunc": {}
}"#);
        fs::write(api_dst_path, api)?;
    }

    let cargo_dst_path = std::env::current_dir()?.join("Cargo.toml");
    if !cargo_dst_path.exists() {
        let mut cargo = String::new();
        cargo.push_str(&format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
"#, crate::get_project_name()?.unwrap()));
        fs::write(cargo_dst_path, cargo)?;
    }

    let rust_src_path = std::env::current_dir()?.join("src");
    if !rust_src_path.exists() {
        let mut src = String::new();
            src.push_str("fn uerust_loaded(is_reload: bool) {\n");
            src.push_str("    logger::init();\n");
            src.push_str("    log::warn!(\"module was {}.\", if is_reload { \"reloaded\" } else { \"loaded\" });\n");
            src.push_str("}\n");
        fs::create_dir_all(&rust_src_path)?;
        fs::write(rust_src_path.join("lib.rs"), src)?;
    }

    let funcs = parse_api_json(api)?;

    // --- Generate ./src/bindings.rs ---
    let mut rust = String::new();
    rust.push_str("// Auto-generated by uer — do not edit\n");
    rust.push_str("extern \"C\" fn rinvoke(rmethod_id: u16, args: *const Argument, num_args: usize) -> Return {\n");
    rust.push_str("    let args = unsafe { std::slice::from_raw_parts(args, num_args) };\n");

    for (idx, func) in funcs.1.iter().enumerate() {
        if idx == 0 { rust.push_str("    "); }
        rust.push_str(&format!("if rmethod_id == RMethodId::{} as u16 {{\n", func.name));
        for (i, (key, arg)) in func.args.iter().enumerate() {
            if arg.is_buf_like {
                rust.push_str(&format!("        let Some({}_data) = args.get({}).map(|v| {{ assert_eq!(v.tag, ArgTag::{}); unsafe {{ v.value.{}_val }} }}) else {{ return Return::none() }};\n", key, i, arg.tag, arg.tag.to_lowercase()));
                let ty = TypeSpec::from_rust_type("usize")?;
                rust.push_str(&format!("        let Some({}_len) = args.get({}).map(|v| {{ assert_eq!(v.tag, ArgTag::{}); unsafe {{ v.value.{}_val }} }}) else {{ return Return::none() }};\n", key, i, ty.tag, ty.tag.to_lowercase()));
            } else {
                rust.push_str(&format!("        let Some({}) = args.get({}).map(|v| {{ assert_eq!(v.tag, ArgTag::{}); unsafe {{ v.value.{}_val }} }}) else {{ return Return::none() }};\n", key, i, arg.tag, arg.tag.to_lowercase()));
            }
        }
        let params = func.args.iter()
            .map(|(k, v)| if v.is_buf_like { format!("{0}_data: {1}, {0}_len: usize", k, v.rust_macro_type) } else { format!("{}: {}", k, v.rust_macro_type) })
            .collect::<Vec<String>>().join(", ");
        let ret = if func.ret.rust_type == "()" { "".to_owned() } else { format!("-> {} ", func.ret.rust_macro_type) };
        rust.push_str(&format!("        // uerust_ffi!(fn {}({}) {}{{ unimplemented!() }});\n", func.name, params, ret));
        let args = func.args.iter()
            .map(|(k, v)| if v.is_buf_like { format!("{0}_data, {0}_len", k) } else { format!("{}", k) })
            .collect::<Vec<String>>().join(", ");
        if func.ret.rust_type == "()" {
            rust.push_str(&format!("        crate::{}({});\n", func.name, args));
            rust.push_str("        return Return::none();\n");
        } else {
            rust.push_str(&format!("        return Return::some(Argument {{ tag: ArgTag::{}, value: ArgType {{ {}_val: crate::{}({}) }} }});\n", func.ret.tag, func.ret.tag.to_lowercase(), func.name, args));
        }
        if idx == funcs.1.len() - 1 {
            rust.push_str("    }\n");
        } else {
            rust.push_str("    } else ");
        }
    }

    rust.push_str("    Return::none()\n");
    rust.push_str("}\n\n");

    rust.push_str("#[allow(non_snake_case)] #[repr(C)] pub struct UERustRsApi {\n");
    rust.push_str("    pub rinvoke: extern \"C\" fn(rmethod_id: u16, args: *const Argument, num_args: usize) -> Return,\n");
    rust.push_str("}\n\n");

    rust.push_str("#[allow(non_snake_case)] #[repr(C)] pub struct UERustCApi {\n");
    rust.push_str("    pub invoke: extern \"C\" fn(method_id: u16, args: *const Argument, num_args: usize) -> Return,\n");
    rust.push_str("}\n\n");

    rust.push_str("#[allow(non_snake_case)] static CAPI: std::sync::OnceLock<UERustCApi> = std::sync::OnceLock::new();\n");
    rust.push_str("#[allow(dead_code)] #[allow(non_snake_case)] fn GetUERustCApi() -> &'static UERustCApi { CAPI.get().expect(\"uerust capi not initialized\") }\n");
    rust.push_str("#[allow(non_snake_case)] #[unsafe(no_mangle)] pub extern \"C\" fn GetUERustRsApi_0(capi: UERustCApi) -> UERustRsApi {\n");
    rust.push_str("    CAPI.get_or_init(|| capi);\n");
    rust.push_str("    UERustRsApi {\n");
    rust.push_str("        rinvoke,\n");
    rust.push_str("    }");
    rust.push_str("}\n\n");

    rust.push_str("use std::mem::ManuallyDrop;\n\n");

    rust.push_str("#[repr(C)]\n");
    rust.push_str("pub struct Buffer {\n");
    rust.push_str("    pub ptr: *const u8,\n");
    rust.push_str("    pub len: usize,\n");
    rust.push_str("}\n");

    rust.push_str("#[derive(PartialEq, Eq, Debug)]\n");
    rust.push_str("#[allow(unused)]\n");
    rust.push_str("#[repr(u8)]\n");
    rust.push_str("pub enum ArgTag {\n");
    rust.push_str("    Unit, Bool, U8, I8, U16, I16, U32, I32, U64, I64, F32, F64, F64CPtr, F64Ptr, USize, ISize, Void, CPtr, Ptr, Buf, String,\n");
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
    rust.push_str("    u64_val: u64,\n");
    rust.push_str("    i64_val: i64,\n");
    rust.push_str("    f32_val: f32,\n");
    rust.push_str("    f64_val: f64,\n");
    rust.push_str("    f64cptr_val: *const f64,\n");
    rust.push_str("    f64ptr_val: *mut f64,\n");
    rust.push_str("    usize_val: usize,\n");
    rust.push_str("    isize_val: isize,\n");
    rust.push_str("    void_val: *mut std::ffi::c_void,\n");
    rust.push_str("    cptr_val: *const std::ffi::c_void,\n");
    rust.push_str("    ptr_val: *mut std::ffi::c_void,\n");
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
    rust.push_str("#[allow(dead_code)]\n");
    rust.push_str("impl Return {\n");
    rust.push_str("    pub fn none() -> Self {\n");
    rust.push_str("        Self { is_some: false, data: Argument { tag: ArgTag::Bool, value: ArgType { bool_val: false } } }\n");
    rust.push_str("    }\n");
    rust.push_str("    pub fn some(data: Argument) -> Self { Self { is_some: true, data } }\n");
    rust.push_str("}\n");

    if !funcs.0.is_empty() {
        rust.push_str("#[allow(non_camel_case_types)]\n");
        rust.push_str("#[repr(C)]\n");
        rust.push_str("enum MethodId {\n");
        for (i, func) in funcs.0.iter().enumerate() {
            let name = &func.name;
            rust.push_str(&format!("    {name} = {},\n", i + 1));
        }
        rust.push_str("}\n");
        rust.push_str("\n");
    }

    if !funcs.1.is_empty() {
        rust.push_str("#[allow(non_camel_case_types)]\n");
        rust.push_str("#[repr(C)]\n");
        rust.push_str("enum RMethodId {\n");
        for (i, func) in funcs.1.iter().enumerate() {
            let name = &func.name;
            rust.push_str(&format!("    {name} = {},\n", i + 1));
        }
        rust.push_str("}\n");
        rust.push_str("\n");
    }

    for func in &funcs.0 {
        let name = &func.name;

        rust.push_str(&format!("/// {} — {}\n", name, func.doc));
        rust.push_str(&format!("#[inline]\npub fn invoke_{}(", name));

        for (i, (key, arg)) in func.args.iter().enumerate() {
            if i == func.args.len() - 1 {
                rust.push_str(&format!("{}: {}", key, arg.rust_type));
            } else {
                rust.push_str(&format!("{}: {}, ", key, arg.rust_type));
            }
        }

        rust.push_str(") -> Return {\n");

        rust.push_str("    let args = [\n");
        for (key, arg) in func.args.iter() {
            if arg.is_buf_like {
                rust.push_str(&format!("        Argument {{ tag: ArgTag::Buf, value: ArgType {{ buf_val: ManuallyDrop::new(Buffer {{ ptr: {0}.as_ptr(), len: {0}.len() }}) }} }},\n", key));
            } else {
                rust.push_str(&format!("        Argument {{ tag: ArgTag::{}, value: ArgType {{ {}_val: {} }} }},\n", arg.tag, arg.rust_type, key));
            }
        }
        rust.push_str("    ];\n");
        rust.push_str(&format!("    (GetUERustCApi().invoke)(MethodId::{name} as u16, (&args).as_ptr(), args.len())\n"));
        rust.push_str("}\n");
    }
    
    rust.push_str(r#"
#[macro_export]
macro_rules! uerust_ffi {
    // =========================================================================
    // Entry Points: Catch the initial function layout
    // =========================================================================
    
    // Pattern 1: Function returns an owned Box
    (fn $name:ident ($($args:tt)*) -> Box<$ret:ty> $body:block) => {
        uerust_ffi!(@munch
            [ $($args)* ] [] [] [] []
            [ @kind [ box_ret ] @name [ $name ] @body [ $body ] ]
        );
    };

    // Pattern 2: Standard or empty/void return function
    (fn $name:ident ($($args:tt)*) $body:block) => {
        uerust_ffi!(@munch
            [ $($args)* ] [] [] [] []
            [ @kind [ normal_ret ] @name [ $name ] @body [ $body ] ]
        );
    };

    // Pattern 3: Standard or empty/void return function
    (fn $name:ident ($($args:tt)*) -> $ret:ty $body:block) => {
        uerust_ffi!(@munch
            [ $($args)* ] [] [] [] []
            [ @kind [ $ret ] @name [ $name ] @body [ $body ] ]
        );
    };

    // =========================================================================
    // TT Muncher: Processes arguments one-by-one (ordered specific to general)
    // =========================================================================

    // Case A: &mut Box<T>
    (@munch [ $obj:ident : &mut Box<$inner:ty> $(, $($rest:tt)*)? ]
            [ $($c_args:tt)* ] [ $($preamble:tt)* ] [ $($cl_params:tt)* ] [ $($cl_args:tt)* ]
            [ $($meta:tt)* ]) => {
        uerust_ffi!(@munch [ $($($rest)*)? ]
            [ $($c_args)* vptr: *mut std::ffi::c_void, ]
            [ $($preamble)* let $obj = unsafe { (vptr as *mut $inner).as_mut() }.unwrap(); ]
            [ $($cl_params)* ] // Captured implicitly by the closure environment
            [ $($cl_args)* ]
            [ $($meta)* ]);
    };

    // Case B: &Box<T>
    (@munch [ $obj:ident : &Box<$inner:ty> $(, $($rest:tt)*)? ]
            [ $($c_args:tt)* ] [ $($preamble:tt)* ] [ $($cl_params:tt)* ] [ $($cl_args:tt)* ]
            [ $($meta:tt)* ]) => {
        uerust_ffi!(@munch [ $($($rest)*)? ]
            [ $($c_args)* vptr: *const std::ffi::c_void, ]
            [ $($preamble)* let $obj = unsafe { (vptr as *const $inner).as_ref() }.unwrap(); ]
            [ $($cl_params)* ] // Captured implicitly by the closure environment
            [ $($cl_args)* ]
            [ $($meta)* ]);
    };

    // Case C: Box<T> (Owned value passing, typically for drop signatures)
    (@munch [ $obj:ident : Box<$inner:ty> $(, $($rest:tt)*)? ]
            [ $($c_args:tt)* ] [ $($preamble:tt)* ] [ $($cl_params:tt)* ] [ $($cl_args:tt)* ]
            [ $($meta:tt)* ]) => {
        uerust_ffi!(@munch [ $($($rest)*)? ]
            [ $($c_args)* vptr: *mut std::ffi::c_void, ]
            [ $($preamble)* let $obj = if vptr.is_null() { return; } else { unsafe { Box::from_raw(vptr as *mut $inner) } }; ]
            [ $($cl_params)* $obj: Box<$inner>, ]
            [ $($cl_args)* $obj, ]
            [ $($meta)* ]);
    };

    // Case D: &mut T (Standard mutable pointer reference)
    (@munch [ $name:ident : &mut $ty:ty $(, $($rest:tt)*)? ]
            [ $($c_args:tt)* ] [ $($preamble:tt)* ] [ $($cl_params:tt)* ] [ $($cl_args:tt)* ]
            [ $($meta:tt)* ]) => {
        uerust_ffi!(@munch [ $($($rest)*)? ]
            [ $($c_args)* $name: *mut $ty, ]
            [ $($preamble)* let $name = unsafe { ($name as *mut $ty).as_mut() }.unwrap(); ]
            [ $($cl_params)* $name: &mut $ty, ]
            [ $($cl_args)* $name, ]
            [ $($meta)* ]);
    };

    // Case E: &T (Standard constant pointer reference)
    (@munch [ $name:ident : &$ty:ty $(, $($rest:tt)*)? ]
            [ $($c_args:tt)* ] [ $($preamble:tt)* ] [ $($cl_params:tt)* ] [ $($cl_args:tt)* ]
            [ $($meta:tt)* ]) => {
        uerust_ffi!(@munch [ $($($rest)*)? ]
            [ $($c_args)* $name: *const $ty, ]
            [ $($preamble)* let $name = unsafe { ($name as *const $ty).as_ref() }.unwrap(); ]
            [ $($cl_params)* $name: &$ty, ]
            [ $($cl_args)* $name, ]
            [ $($meta)* ]);
    };

    // Case F: Plain T (Primitive data types like bool, f64 passed by value)
    (@munch [ $name:ident : $ty:ty $(, $($rest:tt)*)? ]
            [ $($c_args:tt)* ] [ $($preamble:tt)* ] [ $($cl_params:tt)* ] [ $($cl_args:tt)* ]
            [ $($meta:tt)* ]) => {
        uerust_ffi!(@munch [ $($($rest)*)? ]
            [ $($c_args)* $name: $ty, ]
            [ $($preamble)* ]
            [ $($cl_params)* $name: $ty, ]
            [ $($cl_args)* $name, ]
            [ $($meta)* ]);
    };

    // =========================================================================
    // Base Cases: Terminal emission rules when argument queue is empty `[]`
    // =========================================================================

    // Emit rule for normal functions
    (@munch [ ]
            [ $($c_args:tt)* ] [ $($preamble:tt)* ] [ $($cl_params:tt)* ] [ $($cl_args:tt)* ]
            [ @kind [ normal_ret ] @name [ $name:ident ] @body [ $body:block ] ]) => {
        pub extern "C" fn $name ( $($c_args)* ) {
            $($preamble)*
            $body
        }
    };

    // Emit rule for Box factory constructors
    (@munch [ ]
            [ $($c_args:tt)* ] [ $($preamble:tt)* ] [ $($cl_params:tt)* ] [ $($cl_args:tt)* ]
            [ @kind [ box_ret ] @name [ $name:ident ] @body [ $body:block ] ]) => {
        pub extern "C" fn $name ( $($c_args)* ) -> *mut std::ffi::c_void {
            $($preamble)*
            Box::into_raw(Box::new($body)) as *mut std::ffi::c_void
        }
    };

    // Emit rule for result functions
    (@munch [ ]
            [ $($c_args:tt)* ] [ $($preamble:tt)* ] [ $($cl_params:tt)* ] [ $($cl_args:tt)* ]
            [ @kind [ $ret:ty ] @name [ $name:ident ] @body [ $body:block ] ]) => {
        pub extern "C" fn $name ( $($c_args)* ) -> $ret {
            $($preamble)*
            $body
        }
    };
}
"#);

    fs::create_dir_all(rs_output_dir)?;
    fs::write(rs_output_dir.join("bindings.rs"), rust)?;

    if let Some(dir) = project_module_dir {

        let mut h = String::new();
        h.push_str("// Auto-generated by uer — do not edit\n");
        h.push_str("#pragma once\n\n");
        h.push_str("#include \"CoreMinimal.h\"\n");
        h.push_str("#include \"Modules/ModuleManager.h\"\n");
        h.push_str("#include \"bindings.h\"\n\n");

        h.push_str("class UUERustPluginEngineSubsystem;\n\n");
        h.push_str("struct UERustApi {\n");
        h.push_str("private:\n");
        h.push_str("    UUERustPluginEngineSubsystem* subsystem;\n");
        h.push_str("public:\n");
        h.push_str("    UERustApi(UUERustPluginEngineSubsystem& s);\n");
        for func in funcs.1.iter() {
            let args = func.args.iter().map(|(k, v)| format!("{} {}", v.c_type, k)).collect::<Vec<_>>().join(", ");
            h.push_str(&format!("    {} {}({});\n", func.ret.c_type, func.name, args));
        }
        h.push_str("};\n\n");

        h.push_str("class FUERustModuleImpl : public FDefaultGameModuleImpl\n");
        h.push_str("{\n");
        h.push_str("public:\n");
        h.push_str("    TOptional<UERustApi> RApi;\n");
        h.push_str("public:\n");
        h.push_str("    virtual ~FUERustModuleImpl() {}\n\n");
        h.push_str("    virtual void StartupModule() override;\n");
        h.push_str("    virtual void ShutdownModule() override;\n");
        h.push_str("protected:\n");

        for func in &funcs.0 {
            let params = func.args.iter().map(|(_, ty)| 
                if ty.is_buf_like {
                    Ok(vec![format!("{}", ty.c_type), format!("{}", TypeSpec::from_rust_type("usize")?.c_type)])
                } else {
                    Ok(vec![format!("{}", ty.c_type)])
                }
            ).collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?.into_iter().flat_map(|v| v)
            .enumerate().map(|(i, v)| format!("{v} a{i}"))
            .collect::<Vec<_>>().join(", ");
            let ret = if func.ret.rust_type == "()" { "void" } else { func.ret.c_type };
            if func.name == "uerust_log" {
                h.push_str(&format!("    virtual {} {}({});\n", ret, func.name, params));
            } else {
                h.push_str(&format!("    virtual {} {}({}) = 0;\n", ret, func.name, params));
            }
        }

        h.push_str("};\n");

        let pub_dir = dir.join("Public");
        fs::create_dir_all(&pub_dir)?;
        fs::write(pub_dir.join("UERustModuleImpl.h"), h)?;

        let mut cpp = String::new();
        cpp.push_str("// Auto-generated by uer — do not edit\n");
        cpp.push_str("#include \"UERustModuleImpl.h\"\n");
        cpp.push_str("#include \"Modules/ModuleManager.h\"\n");
        cpp.push_str("#include \"Misc/CoreDelegates.h\"\n");
        cpp.push_str("#include \"Engine/Engine.h\"\n");
        cpp.push_str("#include \"UERustPluginEngineSubsystem.h\"\n\n");

        if !funcs.0.is_empty() {
            cpp.push_str("enum class MethodId {\n");
            for (i, func) in funcs.0.iter().enumerate() {
                let name = &func.name;
                cpp.push_str(&format!("    {name} = {},\n", i + 1));
            }
            cpp.push_str("};\n\n");
        }

        if !funcs.1.is_empty() {
            cpp.push_str("enum class RMethodId {\n");
            for (i, func) in funcs.1.iter().enumerate() {
                let name = &func.name;
                cpp.push_str(&format!("    {name} = {},\n", i + 1));
            }
            cpp.push_str("};\n\n");
        }

        cpp.push_str("void FUERustModuleImpl::StartupModule()\n");
        cpp.push_str("{\n");
        cpp.push_str("    FDefaultGameModuleImpl::StartupModule();\n\n");

        cpp.push_str("    FCoreDelegates::OnPostEngineInit.AddLambda([this]() {\n");
        cpp.push_str("        if (GEngine) {\n");
        cpp.push_str("            if (UUERustPluginEngineSubsystem* Subsystem = GEngine->GetEngineSubsystem<UUERustPluginEngineSubsystem>()) {\n");
        cpp.push_str("                Subsystem->SetOnLoaded([this](UUERustPluginEngineSubsystem& subsystem, bool isReload) {\n");
        cpp.push_str("                    RApi.Emplace(UERustApi(subsystem));\n");
        cpp.push_str("                    if (RApi.IsSet()) {\n");
        cpp.push_str("                        RApi.GetValue().uerust_loaded(isReload);\n");
        cpp.push_str("                    }\n\n");
                        
        cpp.push_str("                    subsystem.OnInvoke = [this](uint16_t method_id, Argument const* args, size_t num_args) {\n");

        for (idx, func) in funcs.0.iter().enumerate() {
            if idx == 0 { cpp.push_str("                        "); }
            cpp.push_str(&format!("if ((MethodId)method_id == MethodId::{}) {{\n", func.name));
            for (i, (key, arg)) in func.args.iter().enumerate() {
                if arg.is_buf_like {
                    cpp.push_str(&format!("                            {} {}_data = args[{}].value.buf_val.ptr;\n", arg.c_type, key, i));
                    cpp.push_str(&format!("                            {} {}_len = args[{}].value.buf_val.len;\n", TypeSpec::from_rust_type("usize")?.c_type, key, i));
                } else {
                    cpp.push_str(&format!("                            {} const& {} = args[{}].value.{}_val;\n", arg.c_type, key, i, arg.rust_type));
                }
            }

            let args = func.args.iter().map(|(k, v)| if v.is_buf_like { format!("{0}_data, {0}_len", k) } else { format!("{}", k) }).collect::<Vec<String>>().join(", ");
            if func.ret.rust_type == "()" {
                cpp.push_str(&format!("                            {}({});\n", func.name, args));
                cpp.push_str("                            return Return { .is_some = false };\n");
            } else {
                cpp.push_str(&format!("                            return Return {{ .is_some = true, .value = Argument {{ .tag = ArgTag::{}, .value = ArgType {{ .{}_val = uerust_{}({}) }} }} }};\n", func.ret.tag, func.ret.rust_type, func.name, args));
            }
            if idx == funcs.0.len() - 1 {
                cpp.push_str("                        }\n");
            } else {
                cpp.push_str("                        } else ");
            }
        }

        cpp.push_str("                        return Return { .is_some = false };\n");
        cpp.push_str("                    };\n");
        cpp.push_str("                });\n");
        cpp.push_str("            }\n");
        cpp.push_str("        }\n");
        cpp.push_str("    });\n");
        cpp.push_str("}\n\n");

        cpp.push_str("void FUERustModuleImpl::ShutdownModule()\n");
        cpp.push_str("{\n");
            cpp.push_str("    FDefaultGameModuleImpl::ShutdownModule();\n");
        cpp.push_str("}\n\n");

        cpp.push_str("UERustApi::UERustApi(UUERustPluginEngineSubsystem& s) : subsystem(&s) {}\n\n");

        for func in funcs.1.iter() {
            let args = func.args.iter().map(|(k, v)| format!("{} {}", v.c_type, k)).collect::<Vec<_>>().join(", ");
            cpp.push_str(&format!("{} UERustApi::{}({}) {{\n", func.ret.c_type, func.name, args));
            if func.args.is_empty() {
                cpp.push_str("    Argument args[] = {};\n");
            } else if func.args.len() == 1 {
                let (key, fst) = &func.args[0];
                cpp.push_str(&format!("    Argument args[] = {{ Argument {{ .tag = ArgTag::{}, .value = ArgType {{ .{}_val = {} }} }}, }};\n", fst.tag, fst.tag.to_lowercase(), key));
            } else {
                cpp.push_str("    Argument args[] = {\n");
                for (key, arg) in func.args.iter() {
                    cpp.push_str(&format!("        Argument {{ .tag = ArgTag::{}, .value = ArgType {{ .{}_val = {} }} }},\n", arg.tag, arg.tag.to_lowercase(), key));
                }
                cpp.push_str("    };\n");
            }
            if func.ret.rust_type == "()" {
                cpp.push_str(&format!("    subsystem->RInvoke((uint16_t)RMethodId::{}, args, {});\n", func.name, func.args.len()));
            } else {
                cpp.push_str(&format!("    return subsystem->RInvoke((uint16_t)RMethodId::{}, args, {}).value.value.{}_val;\n", func.name, func.args.len(), func.ret.tag.to_lowercase()));
            }
            cpp.push_str("}\n");
        }

        cpp.push_str("\n");
        cpp.push_str("void FUERustModuleImpl::uerust_log(uint8_t const* a0, uintptr_t a1, uint8_t lvl)\n");
        cpp.push_str("{\n");
        cpp.push_str("    auto data = (char const*)a0;\n");
        cpp.push_str("    auto len = (int)a1;\n");
        cpp.push_str("    if (lvl == 4) { // Error\n");
        cpp.push_str("        UE_LOG(LogTemp, Error, TEXT(\"%s\"), *FString::Printf(TEXT(\"%.*s\"), len, ANSI_TO_TCHAR(data)));\n");
        cpp.push_str("    } else if (lvl == 3) { // Warn\n");
        cpp.push_str("        UE_LOG(LogTemp, Warning, TEXT(\"%s\"), *FString::Printf(TEXT(\"%.*s\"), len, ANSI_TO_TCHAR(data)));\n");
        cpp.push_str("    } else if (lvl == 2) { // Info \n");
        cpp.push_str("        UE_LOG(LogTemp, Display, TEXT(\"%s\"), *FString::Printf(TEXT(\"%.*s\"), len, ANSI_TO_TCHAR(data)));\n");
        cpp.push_str("    } else if (lvl == 1) { // Debug\n");
        cpp.push_str("        UE_LOG(LogTemp, Log, TEXT(\"%s\"), *FString::Printf(TEXT(\"%.*s\"), len, ANSI_TO_TCHAR(data)));\n");
        cpp.push_str("    } else { // Trace\n");
        cpp.push_str("        UE_LOG(LogTemp, Verbose, TEXT(\"%s\"), *FString::Printf(TEXT(\"%.*s\"), len, ANSI_TO_TCHAR(data)));\n");
        cpp.push_str("    }\n");
        cpp.push_str("}\n\n");


        let priv_dir = dir.join("Private");
        fs::create_dir_all(&priv_dir)?;
        fs::write(priv_dir.join("UERustModuleImpl.cpp"), cpp)?;
    }

    let dst_path = std::env::current_dir()?.join("Plugins").join("UERustPlugin");
    if !dst_path.exists() {
        fs::create_dir_all(&dst_path)?;
        ASSETS.extract(dst_path)?;
    }

    println!("✅ Successfully generated bindings for {} functions", funcs.0.len());
    Ok(())
}
