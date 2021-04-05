use std::sync::Arc;

use vulkano::{device::Device, OomError};

mod clear {
    vulkano_shaders::shader! {
        ty: "compute",
        bytes: "src/clear.comp.spv"
    }
}

mod redex {
    vulkano_shaders::shader! {
        ty: "compute",
        bytes: "src/redex.comp.spv"
    }
}

mod visit {
    vulkano_shaders::shader! {
        ty: "compute",
        bytes: "src/visit.comp.spv"
    }
}

pub struct Kernels {
    pub clear: clear::Shader,
    pub redex: redex::Shader,
    pub visit: redex::Shader,
}

impl Kernels {
    pub fn load(device: Arc<Device>) -> Result<Self, OomError> {
        Ok(Self {
            clear: clear::Shader::load(device.clone())?,
            redex: redex::Shader::load(device.clone())?,
            visit: redex::Shader::load(device)?,
        })
    }
}
