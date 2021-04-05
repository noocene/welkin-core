use std::io::Write;

use shaderc::ResolvedInclude;

fn compile_kernel(entry: &str) {
    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_include_callback(|a, _, _, _| {
        Ok(ResolvedInclude {
            resolved_name: a.to_owned(),
            content: std::fs::read_to_string(format!("src/{}", a)).unwrap(),
        })
    });

    println!("cargo:rerun-if-changed=src/{}.comp.glsl", entry);

    let result = compiler
        .compile_into_spirv(
            std::fs::read_to_string(format!("src/{}.comp.glsl", entry))
                .unwrap()
                .as_str(),
            shaderc::ShaderKind::Compute,
            &format!("{}.comp.glsl", entry),
            entry,
            Some(&options),
        )
        .unwrap();

    let mut kernel = std::fs::File::create(format!("src/{}.comp.spv", entry)).unwrap();
    kernel.write_all(result.as_binary_u8()).unwrap();
    kernel.flush().unwrap();
}

fn main() {
    println!("cargo:rerun-if-changed=src/util.glsl");

    compile_kernel("redex");
    compile_kernel("visit");
}
