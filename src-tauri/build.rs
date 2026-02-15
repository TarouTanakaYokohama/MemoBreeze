use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    configure_google_oauth_env();
    if target_os() == "macos" {
        configure_vosk_linking();
    }
    #[cfg(target_os = "macos")]
    build_macos_audio();
    tauri_build::build()
}

fn target_os() -> String {
    env::var("CARGO_CFG_TARGET_OS").unwrap_or_default()
}

fn configure_google_oauth_env() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.clone());

    let dotenv_path = workspace_root.join(".env");
    let example_path = workspace_root.join(".env.example");
    println!("cargo:rerun-if-changed={}", dotenv_path.display());
    println!("cargo:rerun-if-changed={}", example_path.display());

    propagate_env_or_dotenv("GOOGLE_OAUTH_CLIENT_ID", &dotenv_path);
    propagate_env_or_dotenv("GOOGLE_OAUTH_CLIENT_SECRET", &dotenv_path);
}

fn propagate_env_or_dotenv(key: &str, dotenv_path: &PathBuf) {
    if let Ok(value) = env::var(key) {
        println!("cargo:rustc-env={key}={value}");
        return;
    }

    if let Some(value) = read_dotenv_value(dotenv_path, key) {
        println!("cargo:rustc-env={key}={value}");
    }
}

fn read_dotenv_value(dotenv_path: &PathBuf, key: &str) -> Option<String> {
    let raw = fs::read_to_string(dotenv_path).ok()?;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix(&format!("{key}=")) {
            return Some(rest.trim().trim_matches('"').trim_matches('\'').to_string());
        }
    }
    None
}

fn configure_vosk_linking() {
    if let Ok(dir) = env::var("VOSK_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
        return;
    }

    let target = env::var("TARGET").ok();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut search_dirs = Vec::new();

    if let Some(ref triple) = target {
        if triple.contains("apple-darwin") {
            search_dirs.push(manifest_dir.join("libs/macos"));
        } else if triple.contains("windows") {
            search_dirs.push(manifest_dir.join("libs/windows"));
        } else if triple.contains("linux") {
            search_dirs.push(manifest_dir.join("libs/linux"));
        }
    }

    if let Ok(home) = env::var("HOME") {
        let base = PathBuf::from(home).join("vosk-api-0.3.50");
        let extra = vec![
            base.clone(),
            base.join("src"),
            base.join("src/lib"),
            base.join("python"),
            base.join("python/lib"),
            base.join("python/lib/osx"),
            base.join("python/lib/macos"),
        ];
        search_dirs.extend(extra);
    }

    let lib_name = match target.as_deref() {
        Some(triple) if triple.contains("apple-darwin") => "libvosk.dylib",
        Some(triple) if triple.contains("windows") => "vosk.dll",
        Some(_) => "libvosk.so",
        None => "libvosk.dylib",
    };

    let is_unix_like = target
        .as_deref()
        .map(|triple| triple.contains("apple-darwin") || triple.contains("linux"))
        .unwrap_or(true);

    let target_triple = target.clone().unwrap_or_default();

    if let Some(found) = search_dirs
        .into_iter()
        .find(|dir| dir.join(lib_name).exists())
    {
        if let Some(path_str) = found.to_str() {
            println!("cargo:rustc-link-search=native={path_str}");
            if is_unix_like {
                println!("cargo:rustc-link-arg=-Wl,-rpath,{path_str}");
                println!("cargo:rustc-link-arg-bin=MemoBreeze=-Wl,-rpath,{path_str}");

                if target_triple.contains("apple-darwin") {
                    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
                    println!(
                        "cargo:rustc-link-arg-bin=MemoBreeze=-Wl,-rpath,@executable_path/../Frameworks"
                    );
                }
            }
        }

        let lib_path = found.join(lib_name);

        if target_triple.contains("apple-darwin") {
            if let Some(lib_str) = lib_path.to_str() {
                // Check if the library ID is already set correctly
                let needs_update = match Command::new("otool").args(["-D", lib_str]).output() {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        !stdout.contains("@rpath/libvosk.dylib")
                    }
                    Err(_) => true, // If otool fails, try to update anyway
                };

                if needs_update {
                    match Command::new("install_name_tool")
                        .args(["-id", "@rpath/libvosk.dylib", lib_str])
                        .status()
                    {
                        Ok(status) if status.success() => {}
                        Ok(status) => println!(
                            "cargo:warning=install_name_tool exited with status {status} while updating libvosk.dylib"
                        ),
                        Err(error) => println!(
                            "cargo:warning=Failed to execute install_name_tool for libvosk.dylib: {error}"
                        ),
                    }
                }
            }
        }

        println!("cargo:rerun-if-changed={}", lib_path.display());

        if let Ok(out_dir) = env::var("OUT_DIR") {
            let mut profile_dir = PathBuf::from(&out_dir);
            for _ in 0..3 {
                if let Some(parent) = profile_dir.parent() {
                    profile_dir = parent.to_path_buf();
                }
            }

            let mut targets = vec![
                profile_dir.join(lib_name),
                profile_dir.join("deps").join(lib_name),
            ];

            let bundle_root = profile_dir.join("bundle/macos/MemoBreeze.app/Contents");
            targets.push(bundle_root.join("MacOS").join(lib_name));
            targets.push(bundle_root.join("Frameworks").join(lib_name));

            for dest in targets {
                if let Some(parent) = dest.parent() {
                    let _ = fs::create_dir_all(parent);
                }

                // Only copy if destination doesn't exist or has different size
                let should_copy = match (fs::metadata(&lib_path), fs::metadata(&dest)) {
                    (Ok(src_meta), Ok(dst_meta)) => src_meta.len() != dst_meta.len(),
                    _ => true, // Copy if source doesn't exist or destination doesn't exist
                };

                if should_copy {
                    if let Err(error) = fs::copy(&lib_path, &dest) {
                        println!(
                            "cargo:warning=Failed to copy {lib_name} to {}: {error}",
                            dest.display()
                        );
                    }
                }
            }
        }
    } else {
        println!(
            "cargo:warning=Vosk native library ({lib_name}) not found. Set VOSK_LIB_DIR to the directory containing the library."
        );
    }
}

#[cfg(target_os = "macos")]
fn build_macos_audio() {
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let source = manifest_dir.join("src/system_audio.mm");

    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=14.2");

    let mut build = cc::Build::new();
    build.file(&source);
    build.flag("-fobjc-arc");
    build.cpp(true);
    build.flag("-std=gnu++17");
    build.flag("-mmacosx-version-min=14.2");
    build.compile("memobreeze_system_audio");

    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=CoreAudio");
}
