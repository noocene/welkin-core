use std::sync::Arc;

use thiserror::Error;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::{
        AutoCommandBufferBuilder, BuildError, CommandBuffer, CommandBufferExecError, DispatchError,
    },
    descriptor::{
        descriptor_set::{
            PersistentDescriptorSet, PersistentDescriptorSetBuf, PersistentDescriptorSetBuildError,
            PersistentDescriptorSetError, StdDescriptorPoolAlloc,
        },
        pipeline_layout::PipelineLayout,
        PipelineLayoutAbstract,
    },
    device::{Device, DeviceCreationError, DeviceExtensions, Features, Queue},
    instance::{Instance, InstanceCreationError, InstanceExtensions, PhysicalDevice},
    memory::{
        pool::{PotentialDedicatedAllocation, StdMemoryPoolAlloc},
        DeviceMemoryAllocError,
    },
    pipeline::{ComputePipeline, ComputePipelineCreationError},
    sync::{FlushError, GpuFuture},
    OomError,
};

const BLOCK_SIZE: u32 = 64;

use super::{Agent, AgentType, Index, Net, Port, Slot};

type DescriptorSet = Arc<
    PersistentDescriptorSet<
        (
            (),
            PersistentDescriptorSetBuf<
                Arc<
                    CpuAccessibleBuffer<
                        [Agent<u32>],
                        PotentialDedicatedAllocation<StdMemoryPoolAlloc>,
                    >,
                >,
            >,
        ),
        StdDescriptorPoolAlloc,
    >,
>;

pub struct Accelerated {
    clear: Arc<ComputePipeline<PipelineLayout<kernels::clear::MainLayout>>>,
    set: DescriptorSet,
    queue: Arc<Queue>,
    device: Arc<Device>,
    agents: Arc<CpuAccessibleBuffer<[Agent<u32>]>>,
}

impl Accelerated {
    fn clear(&mut self) -> Result<(), AcceleratedError> {
        let command_buffer = {
            let mut builder =
                AutoCommandBufferBuilder::new(self.device.clone(), self.queue.family())?;

            builder.dispatch(
                [
                    (self.agents.len() as u32 + BLOCK_SIZE - 1) / BLOCK_SIZE,
                    1,
                    1,
                ],
                self.clear.clone(),
                self.set.clone(),
                (),
                None,
            )?;
            builder.build()?
        };

        let finished = command_buffer.execute(self.queue.clone())?;

        finished.then_signal_fence_and_flush()?.wait(None)?;

        let content = self.agents.read().unwrap();
        for (_, val) in content.iter().enumerate() {
            println!("{:?}", val);
        }

        Ok(())
    }
}

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
    #[error("failed to flush pipeline")]
    Flush(#[from] FlushError),
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

        let agents = CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::all(),
            false,
            vec![Agent::new(
                Port::new(Index(0), Slot::Principal),
                Port::new(Index(0), Slot::Principal),
                Port::new(Index(0), Slot::Principal),
                AgentType::Epsilon,
            )]
            .into_iter(),
        )?;

        let kernels = kernels::Kernels::load(device.clone())?;

        let clear = Arc::new(ComputePipeline::new(
            device.clone(),
            &kernels.clear.main_entry_point(),
            &(),
            None,
        )?);

        let set = {
            let layout = clear
                .layout()
                .descriptor_set_layout(0)
                .ok_or(AcceleratedError::DescriptorSetMissing)?;

            Arc::new(
                PersistentDescriptorSet::start(layout.clone())
                    .add_buffer(agents.clone())?
                    .build()?,
            )
        };

        let mut a = Accelerated {
            clear,
            set,
            queue,
            device,
            agents,
        };

        a.clear()?;

        Ok(a)
    }
}
