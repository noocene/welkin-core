use std::sync::Arc;

use vulkano::{device::Device, OomError};

pub mod redex {
    vulkano_shaders::shader! {
        ty: "compute",
        bytes: "src/redex.comp.spv"
    }
}

pub mod visit {
    vulkano_shaders::shader! {
        ty: "compute",
        bytes: "src/visit.comp.spv"
    }
}

pub struct Kernels {
    pub redex: redex::Shader,
    pub visit: visit::Shader,
}

impl Kernels {
    pub fn load(device: Arc<Device>) -> Result<Self, OomError> {
        Ok(Self {
            redex: redex::Shader::load(device.clone())?,
            visit: visit::Shader::load(device)?,
        })
    }
}
