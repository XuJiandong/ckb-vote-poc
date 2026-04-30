use std::process::Command;

fn compile(schema: &str) {
    let out_dir = std::path::PathBuf::from("./src/molecules");
    let mut compiler = molecule_codegen::Compiler::new();
    let result = compiler
        .input_schema_file(schema)
        .generate_code(molecule_codegen::Language::Rust)
        .output_dir(out_dir)
        .run();

    if let Err(err) = result {
        panic!("Failed to compile schema {}: {}", schema, err);
    }
}

fn main() {
    println!("cargo:rerun-if-changed=molecules/types.mol");
    compile("molecules/types.mol");
    let output = Command::new("cargo")
        .arg("fmt")
        .arg("--")
        .arg("src/molecules/types.rs")
        .output()
        .expect("Failed to execute command");

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        panic!("Command failed: {}", error);
    }
}
