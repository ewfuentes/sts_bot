fn main() {
    // Embed the Python library's rpath so the binary can find libpython
    // at runtime without LD_LIBRARY_PATH.
    let config = pyo3_build_config::get();
    if let Some(lib_dir) = &config.lib_dir {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir);
        return;
    }

    // Fall back to querying Python directly
    let python = std::env::var("PYO3_PYTHON").unwrap_or_else(|_| "python3".to_string());
    if let Ok(output) = std::process::Command::new(&python)
        .args(["-c", "import sysconfig; print(sysconfig.get_config_var('LIBDIR'))"])
        .output()
    {
        let lib_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !lib_dir.is_empty() && lib_dir != "None" {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir);
        }
    }
}
