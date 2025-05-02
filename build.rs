use std::{env, path::PathBuf};
fn get_env_var(var: &str) -> Option<String> {
    println!("cargo::rerun-if-env-changed={var}");
    env::var(var).ok()
}

fn get_linker_from_json(json: &[u8]) -> Option<String> {
    use serde_json::*;
    Some(
        from_slice::<serde_json::Value>(json)
            .ok()?
            .as_object()?
            .get("linker")?
            .as_str()?
            .to_owned(),
    )
}

fn get_native_include_flags() -> Result<Vec<String>, String> {
    let rustc = env::var("RUSTC").unwrap();
    let Some(linker) = get_linker_from_json(
        &std::process::Command::new(rustc)
            .arg("-Zunstable-options")
            .arg("--print")
            .arg("target-spec-json")
            .arg("--target")
            .arg(env::var("TARGET").unwrap())
            .output()
            .expect("failed to execute rustc")
            .stdout,
    ) else {
        return Err("failed to get linker from rustc for this HelenOS target".to_owned());
    };

    let Some(output) = std::process::Command::new(&linker)
        .arg("-xc")
        .arg("-E")
        .arg("-v")
        .arg("-")
        .output()
        .ok()
        .and_then(|x| String::from_utf8(x.stderr).ok())
    else {
        return Err(format!(
            "failed to run linker {linker} to detect include paths"
        ));
    };
    let mut lines = output.lines();
    loop {
        let Some(line) = lines.next() else {
            return Err("failed to detect include paths from linker output".to_owned());
        };

        if line.contains("#include <...> search starts here:") {
            break;
        }
    }
    let mut flags = Vec::new();
    while let Some(l) = lines.next() {
        if !l.starts_with(' ') {
            break;
        }
        flags.push(format!("-I{}", l.trim()));
    }
    if flags.is_empty() {
        return Err("failed to detect include paths from linker output".to_owned());
    }
    Ok(flags)
}

fn main() {
    let target = env::var("TARGET").unwrap();
    if !target.contains("helenos") {
        println!("cargo::warning=Not a HelenOS target ({target}), this package will be empty");
        return;
    }

    let Some(include_base) = get_env_var("HELENOS_INCLUDE_BASE") else {
        println!("cargo::error=HELENOS_INCLUDE_BASE not set");
        return;
    };

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let libextern_file = out_path.join("extern.c");

    for lib in [
        "ui", "gfx", "gfxfont", "riff", "memgfx", "display", "console", "congfx", "ipcgfx",
    ] {
        println!("cargo::rustc-link-lib={lib}");
    }

    let include_flags = [
        "ui", "display", "gfx", "input", "c", "console", "output", "gfxfont",
    ]
    .map(|lib| format!("-I{include_base}/lib{lib}"));

    let native_include_flags = match get_native_include_flags() {
        Ok(paths) => paths,
        Err(e) => {
            println!("cargo::error={e}");
            return;
        }
    };

    let builder = bindgen::Builder::default()
        .header("wrapper.h")
        .sort_semantically(true)
        .allowlist_function(
            "\
                ui_(\
                    create|destroy|\
                    control_.*|\
                    window_.*|\
                    wnd_params_init|\
                    wdecor_.*\
                )|\
                display_.*|\
                gfx_.*|\
                pixelmap_.*|\
                fibril_usleep\
                ",
        )
        .allowlist_var("UI_.*")
        .blocklist_type("sysarg_t|errno_t|console_ctrl_t")
        .bitfield_enum("keymod_t")
        .bitfield_enum("ui_wdecor_style_t")
        .bitfield_enum("ui_wdecor_rsztype_t")
        .bitfield_enum("ui_wnd_flags_t")
        .newtype_enum("keycode_t")
        .newtype_enum("pos_event_type_t")
        .newtype_enum("ui_stock_cursor_t")
        .newtype_enum("ui_evclaim_t")
        .newtype_enum("ui_wnd_placement_t")
        .impl_debug(true)
        .wrap_static_fns(true)
        .wrap_static_fns_path(&libextern_file)
        .clang_args(&include_flags)
        .clang_args(&native_include_flags)
        .clang_arg("-nostdinc")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
    let bindings = builder.generate().expect("Unable to generate bindings");

    let mut libextern_build = cc::Build::new();
    libextern_build
        .file(&libextern_file)
        .include(env::var("CARGO_MANIFEST_DIR").unwrap());
    for flag in include_flags {
        libextern_build.flag(&flag);
    }
    for flag in native_include_flags {
        libextern_build.flag(&flag);
    }
    libextern_build.compile("extern");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
