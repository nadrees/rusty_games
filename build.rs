use std::{
    fs,
    io::{self, Write},
    process::Command,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=shaders/");

    fs::create_dir_all("target/shaders")?;

    let result = Command::new("glslc")
        .arg("shaders/shader.frag")
        .arg("-o")
        .arg("target/shaders/frag.spv")
        .output()?;
    io::stdout().write_all(&result.stdout)?;
    io::stderr().write_all(&result.stderr)?;

    let result = Command::new("glslc")
        .arg("shaders/shader.vert")
        .arg("-o")
        .arg("target/shaders/vert.spv")
        .output()?;
    io::stdout().write_all(&result.stdout)?;
    io::stderr().write_all(&result.stderr)?;

    Ok(())
}
