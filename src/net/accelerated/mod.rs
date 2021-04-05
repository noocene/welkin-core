use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{
        AutoCommandBufferBuilder, BuildError, CommandBuffer, CommandBufferExecError, DispatchError,
    },
    descriptor::{
        descriptor_set::{
            PersistentDescriptorSet, PersistentDescriptorSetBuildError,
            PersistentDescriptorSetError,
        },
        PipelineLayoutAbstract,
    },
    device::{Device, DeviceCreationError, DeviceExtensions, Features},
    instance::{Instance, InstanceCreationError, InstanceExtensions, PhysicalDevice},
    memory::DeviceMemoryAllocError,
    pipeline::{ComputePipeline, ComputePipelineCreationError},
    sync::GpuFuture,
    OomError,
};

const BLOCK_SIZE: u32 = 256;

use super::Net;

pub struct Accelerated {}

impl Accelerated {}

#[derive(Debug, Error)]
pub enum AcceleratedError {
    #[error("error creating Vulkan instance")]
    InstanceCreation(#[from] InstanceCreationError),
    #[error("no suitable Vulkan device")]
    NoSuitableDevice,
    #[error("error creating Vulkan device")]
    DeviceCreation(#[from] DeviceCreationError),
    #[error("out of memory")]
    OutOfMemory(#[from] OomError),
    #[error("error creating compute pipeline")]
    PipelineCreation(#[from] ComputePipelineCreationError),
    #[error("failed to allocate memory on device")]
    DeviceAlloc(#[from] DeviceMemoryAllocError),
    #[error("failed to add buffer to descriptor set")]
    DescriptorSetAdd(#[from] PersistentDescriptorSetError),
    #[error("failed to build descriptor set")]
    DescriptorSetBuild(#[from] PersistentDescriptorSetBuildError),
    #[error("descriptor set 0 is missing")]
    DescriptorSetMissing,
    #[error("failed to build command buffer")]
    CommandBufferBuild(#[from] BuildError),
    #[error("failed to dispatch kernel")]
    Dispatch(#[from] DispatchError),
    #[error("failed to execute kernel")]
    Exec(#[from] CommandBufferExecError),
}

impl Net<u32> {
    pub fn into_accelerated(self) -> Result<Accelerated, AcceleratedError> {
        let instance = Instance::new(None, &InstanceExtensions::none(), None)?;
        let physical = PhysicalDevice::enumerate(&instance)
            .next()
            .ok_or(AcceleratedError::NoSuitableDevice)?;

        let queue_family = physical
            .queue_families()
            .find(|&q| q.supports_compute())
            .ok_or(AcceleratedError::NoSuitableDevice)?;

        let (device, mut queues) = {
            Device::new(
                physical,
                &Features::none(),
                &DeviceExtensions::none(),
                [(queue_family, 0.5)].iter().cloned(),
            )?
        };

        let queue = queues.next().ok_or(AcceleratedError::NoSuitableDevice)?;

        let data_buffer =
            CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, 0..65535)?;

        let kernels = kernels::Kernels::load(device.clone())?;

        let compute_pipeline = Arc::new(ComputePipeline::new(
            device.clone(),
            &kernels.clear.main_entry_point(),
            &(),
            None,
        )?);

        let layout = compute_pipeline
            .layout()
            .descriptor_set_layout(0)
            .ok_or(AcceleratedError::DescriptorSetMissing)?;

        let set = Arc::new(
            PersistentDescriptorSet::start(layout.clone())
                .add_buffer(data_buffer.clone())?
                .build()?,
        );

        let mut builder = AutoCommandBufferBuilder::new(device.clone(), queue.family())?;

        builder.dispatch(
            [(65535 + 1) / BLOCK_SIZE, 1, 1],
            compute_pipeline.clone(),
            set.clone(),
            (),
            None,
        )?;

        let command_buffer = builder.build()?;

        let finished = command_buffer.execute(queue.clone())?;

        finished
            .then_signal_fence_and_flush()
            .unwrap()
            .wait(None)
            .unwrap();

        let content = data_buffer.read().unwrap();
        for (_, val) in content.iter().enumerate() {
            print!("{} ", val);
        }

        println!("\nEverything succeeded!");

        Ok(Accelerated {})
    }
}
