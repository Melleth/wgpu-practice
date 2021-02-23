use anyhow::*;
use glob::glob;
use std::fs::{read_to_string, write};
use std::path::PathBuf;
use std::env;
use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;

struct ShaderData {
    src: String,
    src_path: PathBuf,
    spv_path: PathBuf,
    kind: shaderc::ShaderKind,
}

impl ShaderData {
    pub fn load(src_path: PathBuf) -> Result<Self> {
        let extension = src_path
            .extension()
            .context("File has no extension")?
            .to_str()
            .context("Extension cannot be converted to &str")?;
        
        let kind = match extension {
            "vert" => shaderc::ShaderKind::Vertex,
            "frag" => shaderc::ShaderKind::Fragment,
            "comp" => shaderc::ShaderKind::Compute,
            _ => bail!("Unsupported shader: {}", src_path.display()),
        };

        let src = read_to_string(src_path.clone())?;
        let spv_path = src_path.with_extension(format!("{}.spv", extension));

        Ok(Self {
            src,
            src_path,
            spv_path,
            kind,
        })
    }
}

fn main() -> Result<()> {
    // Collect all shaders
    let mut shaders_paths = [
        glob("./src/**/*.vert")?,
        glob("./src/**/*.frag")?,
        glob("./src/**/*.comp")?,
    ];

    let shaders = shaders_paths
        .iter_mut()
        .flatten()
        .map(|glob_result| ShaderData::load(glob_result?))
        .collect::<Vec<Result<_>>>()
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

    let mut compiler = shaderc::Compiler::new().context("Unable to create shader compiler.")?;

    for shader in shaders {
        // Rerun on file change.
        println!("cargo:rerun-if-changed={}", shader.src_path.as_os_str().to_str().unwrap());

        let compiled = compiler.compile_into_spirv(
            &shader.src,
            shader.kind,
            &shader.src_path.to_str().unwrap(),
            "main",
            None,
        )?;

        write(shader.spv_path, compiled.as_binary_u8())?;
    }

    // Copy resources from project folder to build folder for easy access.
    println!("cargo:rerun-if-changed=src/res/*");
    let out_dir = env::var("OUT_DIR")?;
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;
    let mut paths_to_copy = Vec::new();
    paths_to_copy.push("./src/res/");
    copy_items(&paths_to_copy, out_dir, &copy_options)?;
        
    Ok(())
}