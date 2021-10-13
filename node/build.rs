use substrate_build_script_utils::{generate_cargo_keys, rerun_if_git_head_changed};

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

fn write_functional_tests_config_file()
{
    let cargo_manifest_dir = env!("CARGO_MANIFEST_DIR");
    let profile = env::var("PROFILE").unwrap();
    let bin_name = env::var("CARGO_PKG_NAME").unwrap_or("mintlayer-core".to_owned());

    let build_dir = Path::new(cargo_manifest_dir).join("..").join("target").join(profile);
    let build_dir = build_dir.canonicalize().unwrap_or(build_dir);

    // This doesn't work, so we don't use it until this is fixed
    // let exe_path = env::var_os("CARGO_BIN_EXE_mintlayer-core").unwrap();
    // let exe_path = env!("CARGO_BIN_EXE_mintlayer-core");
    // let exe_path = env!("CARGO_BIN_EXE");

    let exe_path = build_dir.join(bin_name);
    let exe_path = exe_path.canonicalize().unwrap_or(exe_path);

    let src_dir = Path::new(&cargo_manifest_dir).join("../");
    let src_dir = src_dir.canonicalize().unwrap_or(src_dir);
    let src_dir = src_dir.to_string_lossy().to_string();
    let src_dir = src_dir.as_str();

    let template_config_file_path =
        Path::new(&cargo_manifest_dir).join("../").join("test").join("config.ini.in");
    let template_config_file_content = fs::read_to_string(template_config_file_path).unwrap();
    let template_config_file_content = str::replace(
        template_config_file_content.as_str(),
        "@abs_top_srcdir@",
        src_dir,
    );
    let template_config_file_content = str::replace(
        template_config_file_content.as_str(),
        "@abs_top_builddir@",
        build_dir.to_string_lossy().to_string().as_str(),
    );
    let template_config_file_content = str::replace(
        template_config_file_content.as_str(),
        "@EXEEXT@",
        exe_path.extension().unwrap_or(OsStr::new("")).to_string_lossy().to_string().as_str(),
    );

    // write the config file
    let dest_path = Path::new(&cargo_manifest_dir).join("../").join("test").join("config.ini");
    fs::write(&dest_path, template_config_file_content).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=PROFILE");
}

fn main() {
    generate_cargo_keys();

    rerun_if_git_head_changed();

    write_functional_tests_config_file();
}
