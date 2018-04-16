use std::{
    process::{
        Command,
        Output,
        Stdio,
        exit
    },
    path::PathBuf,
    env::current_dir,
    io::{ self, prelude::*},
    fs::{ create_dir, File }
};
use cargo_metadata::{ metadata, Package, Metadata };

fn get_manifest_path(mut dir: PathBuf) -> io::Result<PathBuf> {
    dir.push("Cargo.toml");
    if !dir.exists() {
        dir.pop();
        dir.pop();
        return get_manifest_path(dir)
    }
    Ok(dir)
}

fn create_static(mut dir: PathBuf) -> PathBuf {
    dir.push("static/");
    if !dir.exists() {
        create_dir(&dir).expect("Could not create the static directory")
    }
    dir
}

pub fn get_pkg_metadata() -> Metadata {
    let manifest_path = get_manifest_path(current_dir().expect("can't get the current directory")).unwrap();
    metadata(Some(manifest_path.as_path())).expect("Can't get metadata")
}

pub fn get_package(metadata: &Metadata) -> Package {
    
    metadata.packages[0].clone()
}

pub fn get_workspace_dir(metadata: &Metadata) -> PathBuf {

    PathBuf::from(&metadata.workspace_root)
}

fn target_dir(metadata: &Metadata) -> PathBuf {
    PathBuf::from(&metadata.target_directory)
}

fn get_binary_name(pkg: &Package) -> String {
    let name = if !(pkg.targets[0].kind[0] == "bin") {
        pkg.name.replace("-", "_")
        } else {
            pkg.name.clone()
        };
    format!("{}.wasm", name)
}

pub fn cargo_build() -> Output {
    Command::new("cargo")
        .arg("build")
        .arg("--release")
        .args(&["--target", "wasm32-unknown-unknown"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("Could not build the crate")
}



pub fn generate_wasm(metadata: &Metadata, pkg: Package) -> Output {
    let dir = get_workspace_dir(metadata);
    let mut target = target_dir(metadata);
    
    let bin_name = get_binary_name(&pkg);
    target.push("wasm32-unknown-unknown");
    target.push("release");
    target.push(&bin_name);
    if !target.exists() {
        eprintln!("target: {:?} doesn't exist.", target);
        exit(1);
    }
    let out_dir = create_static(dir);
    let out_dir = generate_asset(out_dir, "index.html", b"
<html>
  <head>
    <meta content=\"text/html;charset=utf-8\" http-equiv=\"Content-Type\"/>
  </head>
  <body>
    <script src='./index.js'></script>
  </body>
</html>
    ");
    let index_js = format!("
let wasm = import('./{}');
wasm.then(m => m.main && m.main());    
    ", bin_name.split(".").next().unwrap());
    let out_dir = generate_asset(out_dir, "index.js", index_js.as_bytes());

    let out_dir = generate_asset(out_dir, "package.json", b"
{
  \"scripts\": {
    \"serve\": \"webpack-dev-server\"
  },
  \"devDependencies\": {
    \"webpack\": \"^4.0.1\",
    \"webpack-cli\": \"^2.0.10\",
    \"webpack-dev-server\": \"^3.1.0\"
  }
}
    ");
    let out_dir = generate_asset(out_dir, "webpack.config.js", b"
const path = require('path');

module.exports = {
  entry: \"./index.js\",
  output: {
    path: path.resolve(__dirname, \"dist\"),
    filename: \"index.js\",
  },
  mode: \"development\"
};
    ");
    Command::new("wasm-bindgen")
        .arg(target.to_str().expect("can't set target"))
        .args(&["--out-dir", &out_dir.to_str().expect("can't set outdir")])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("Could not generate wasm.")
}

fn generate_asset(mut dir: PathBuf, asset_name: &str, content: &[u8]) -> PathBuf {
    dir.push(asset_name);
    if !dir.exists() {
        let mut asset = File::create(&dir).expect("Could not create the index.html");
        asset.write(content).expect(&format!("Could not write to {}", asset_name));
    }
    dir.pop();
    dir
}

pub fn wasm_build() {
    if cargo_build().status.success() {
        let metadata = get_pkg_metadata();
        let pkg = get_package(&metadata);
        generate_wasm(&metadata, pkg);
        serve(&metadata);
    }
}

pub fn serve(metadata: &Metadata) {
    let mut out_dir = create_static(get_workspace_dir(metadata));
    out_dir.push("node_modules/");
    if !out_dir.exists() {
        out_dir.pop();
        let current_dir = out_dir.to_str()
            .expect("Couldn't get static path.");
        
        Command::new("npm")
            .arg("i")
            .current_dir(current_dir)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .expect("Could not install npm packages.");
    } else {
        out_dir.pop();
    }
    let current_dir = out_dir.to_str()
            .expect("Couldn't get static path.");
    Command::new("npm")
        .arg("run")
        .arg("serve")
        .current_dir(current_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .expect("Could not run webpack server");

}