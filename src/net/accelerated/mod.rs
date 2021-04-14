use std::sync::Arc;

#[cfg(feature = "renderdoc")]
use rdoc::{RenderDoc, V120};
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
        pipeline_layout::PipelineLayout,
        DescriptorSet, PipelineLayoutAbstract,
    },
    device::{Device, DeviceCreationError, DeviceExtensions, Features, Queue},
    instance::{Instance, InstanceCreationError, InstanceExtensions, PhysicalDevice},
    memory::DeviceMemoryAllocError,
    pipeline::{ComputePipeline, ComputePipelineCreationError},
    sync::{FlushError, GpuFuture},
    OomError,
};

const BLOCK_SIZE: u32 = 64;

use super::{Agent, AgentType, Index, Net, Port, Slot};

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct State {
    agents: u32,
    active_pairs: u32,
    active_pairs_done: u32,
    freed_agents: u32,
    visits_needed: u32,
    visits_done: u32,
    rewrites: u32,
}

pub struct Accelerated {
    redex: Arc<ComputePipeline<PipelineLayout<kernels::redex::MainLayout>>>,
    visit: Arc<ComputePipeline<PipelineLayout<kernels::visit::MainLayout>>>,
    set: Arc<dyn DescriptorSet + Sync + Send>,
    queue: Arc<Queue>,
    device: Arc<Device>,
    agents: Arc<CpuAccessibleBuffer<[Agent<u32>]>>,
    active_agents: Arc<CpuAccessibleBuffer<[Index<u32>]>>,
    freed_agents: Arc<CpuAccessibleBuffer<[Index<u32>]>>,
    _needs_visiting: Arc<CpuAccessibleBuffer<[Index<u32>]>>,
    state: Arc<CpuAccessibleBuffer<State>>,
    #[cfg(feature = "renderdoc")]
    renderdoc: RenderDoc<V120>,
}

impl Accelerated {
    pub fn reduce_all(&mut self) -> Result<usize, AcceleratedError> {
        #[cfg(feature = "renderdoc")]
        {
            self.renderdoc
                .start_frame_capture(std::ptr::null(), std::ptr::null());
        }

        let a = loop {
            // break Ok(0);
            let command_buffer = {
                let mut builder =
                    AutoCommandBufferBuilder::new(self.device.clone(), self.queue.family())?;

                builder.dispatch(
                    [
                        ({
                            let state = self.state.read().unwrap();
                            state.active_pairs
                        } as u32
                            + BLOCK_SIZE
                            - 1)
                            / BLOCK_SIZE,
                        1,
                        1,
                    ],
                    self.redex.clone(),
                    self.set.clone(),
                    (),
                    None,
                )?;
                builder.build()?
            };

            let finished = command_buffer.execute(self.queue.clone())?;

            finished.then_signal_fence_and_flush()?.wait(None)?;

            let command_buffer = {
                let mut builder =
                    AutoCommandBufferBuilder::new(self.device.clone(), self.queue.family())?;

                builder.dispatch(
                    [
                        ({
                            let state = self.state.read().unwrap();
                            state.visits_needed
                        } as u32
                            + BLOCK_SIZE
                            - 1)
                            / BLOCK_SIZE,
                        1,
                        1,
                    ],
                    self.visit.clone(),
                    self.set.clone(),
                    (),
                    None,
                )?;
                builder.build()?
            };

            let finished = command_buffer.execute(self.queue.clone())?;

            finished.then_signal_fence_and_flush()?.wait(None)?;

            let mut state = self.state.write().unwrap();

            if state.active_pairs == 0 {
                let rewrites = state.rewrites;
                state.rewrites = 0;

                break Ok(rewrites as usize);
            }
        };

        #[cfg(feature = "renderdoc")]
        {
            self.renderdoc
                .end_frame_capture(std::ptr::null(), std::ptr::null());
            self.renderdoc.launch_replay_ui(true, None).unwrap();
            loop {}
        }

        a
    }

    pub fn into_inner(self) -> Net<u32> {
        let mut agents = self.agents.read().unwrap().to_vec();
        let mut freed = self.freed_agents.read().unwrap().to_vec();
        let mut active = self.active_agents.read().unwrap().to_vec();

        let state = &*self.state.read().unwrap();
        freed.truncate(state.freed_agents as usize);
        active.truncate(state.active_pairs as usize);
        agents.truncate(state.agents as usize);

        Net {
            agents,
            freed,
            active,
        }
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
        #[cfg(feature = "renderdoc")]
        let renderdoc = RenderDoc::<V120>::new().unwrap();

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

        let agents_len = self.agents.len();
        let freed_len = self.freed.len();
        let active_len = self.active.len();

        let usage = BufferUsage::all();

        let mut agents = self.agents;
        agents.extend(
            vec![
                Agent::new(
                    Port::new(Index(0), Slot::Principal),
                    Port::new(Index(0), Slot::Principal),
                    Port::new(Index(0), Slot::Principal),
                    AgentType::Wire
                );
                agents_len
            ]
            .into_iter(),
        );

        let mut active = self.active;
        active.extend(vec![Index(0); agents_len]);

        let agents =
            CpuAccessibleBuffer::from_iter(device.clone(), usage, false, agents.into_iter())?;

        let active_agents =
            CpuAccessibleBuffer::from_iter(device.clone(), usage, false, active.into_iter())?;

        let mut freed = self.freed;
        freed.extend(vec![
            Index(std::u32::MAX);
            agents_len.saturating_sub(freed_len)
        ]);

        let freed_agents =
            CpuAccessibleBuffer::from_iter(device.clone(), usage, false, freed.into_iter())?;

        let _needs_visiting = CpuAccessibleBuffer::from_iter(
            device.clone(),
            usage,
            false,
            vec![Index(0); agents_len].into_iter(),
        )?;

        let state = CpuAccessibleBuffer::from_data(
            device.clone(),
            usage,
            false,
            State {
                agents: agents_len as u32,
                active_pairs: active_len as u32,
                active_pairs_done: 0,
                freed_agents: freed_len as u32,
                visits_needed: 0,
                visits_done: 0,
                rewrites: 0,
            },
        )?;

        let kernels = kernels::Kernels::load(device.clone())?;

        let redex = Arc::new(ComputePipeline::new(
            device.clone(),
            &kernels.redex.main_entry_point(),
            &(),
            None,
        )?);

        let visit = Arc::new(ComputePipeline::new(
            device.clone(),
            &kernels.visit.main_entry_point(),
            &(),
            None,
        )?);

        let set = {
            let layout = redex
                .layout()
                .descriptor_set_layout(0)
                .ok_or(AcceleratedError::DescriptorSetMissing)?;

            Arc::new(
                PersistentDescriptorSet::start(layout.clone())
                    .add_buffer(agents.clone())?
                    .add_buffer(active_agents.clone())?
                    .add_buffer(freed_agents.clone())?
                    .add_buffer(_needs_visiting.clone())?
                    .add_buffer(state.clone())?
                    .build()?,
            )
        };

        Ok(Accelerated {
            redex,
            visit,
            set,
            #[cfg(feature = "renderdoc")]
            renderdoc,
            queue,
            state,
            device,
            active_agents,
            freed_agents,
            _needs_visiting,
            agents,
        })
    }
}
