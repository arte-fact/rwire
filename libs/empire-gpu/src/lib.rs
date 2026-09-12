//! The arena on the GPU: whole game years in a compute kernel, one table
//! per lane, so that thousands of brains play at once. The rules are
//! those of `empire-lib`, ported to WGSL and checked against them
//! (`tests/`); the dice are the same xoshiro128++, rolled in the same
//! order.

pub mod layout;
pub mod state;

use std::sync::Arc;

use wgpu::util::DeviceExt;

/// The device the kernels run on.
pub struct Gpu {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub name: String,
}

impl Gpu {
    /// The first GPU found; `None` when there is none.
    pub fn open() -> Option<Gpu> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .ok()?;
        let name = adapter.get_info().name;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("empire"),
            required_features: wgpu::Features::SUBGROUP,
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .ok()?;
        Some(Gpu {
            device: Arc::new(device),
            queue: Arc::new(queue),
            name,
        })
    }

    /// A compute pipeline over the shader `source` (the brain's widths
    /// prepended), its bindings laid out from the source.
    pub fn pipeline(&self, label: &str, source: &str) -> wgpu::ComputePipeline {
        let source = format!("{}\n{source}", layout::header());
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        self.device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: None,
                module: &module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            })
    }

    /// A storage buffer holding `data`, read by the kernels.
    pub fn upload<T: bytemuck::Pod>(&self, label: &str, data: &[T]) -> wgpu::Buffer {
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(data),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            })
    }

    /// A storage buffer of `len` words the kernels write and the CPU reads
    /// back (see [`Gpu::download`]).
    pub fn output(&self, label: &str, len: usize) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (len * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Run `pipeline` over `workgroups` workgroups on `bindings`, in order.
    pub fn dispatch(
        &self,
        pipeline: &wgpu::ComputePipeline,
        bindings: &[&wgpu::Buffer],
        workgroups: u32,
    ) {
        let entries: Vec<wgpu::BindGroupEntry> = bindings
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }
        self.queue.submit([encoder.finish()]);
    }

    /// The words of an output buffer, once the kernels are done.
    pub fn download<T: bytemuck::Pod>(&self, buffer: &wgpu::Buffer) -> Vec<T> {
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: buffer.size(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, buffer.size());
        self.queue.submit([encoder.finish()]);
        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device answers");
        rx.recv().unwrap().expect("the buffer maps");
        let out = bytemuck::cast_slice(&slice.get_mapped_range()).to_vec();
        staging.unmap();
        out
    }
}

/// The shader sources, the common parts first.
pub mod shaders {
    pub const NET: &str = include_str!("wgsl/net.wgsl");
    pub const FORWARD_TEST: &str = include_str!("wgsl/forward_test.wgsl");
    pub const RNG: &str = include_str!("wgsl/rng.wgsl");
    pub const STATE: &str = include_str!("wgsl/state.wgsl");
    pub const SIGHT: &str = include_str!("wgsl/sight.wgsl");
    pub const INTENDANCE: &str = include_str!("wgsl/intendance.wgsl");
    pub const EXTERIEUR: &str = include_str!("wgsl/exterieur.wgsl");
    pub const CAMPAIGN: &str = include_str!("wgsl/campaign.wgsl");
    pub const YEAR: &str = include_str!("wgsl/year.wgsl");

    /// The forward test kernel, whole.
    pub fn forward_test() -> String {
        format!("{FORWARD_TEST}\n{NET}")
    }

    /// The year kernel, whole.
    pub fn year() -> String {
        [
            STATE, RNG, NET, SIGHT, INTENDANCE, EXTERIEUR, CAMPAIGN, YEAR,
        ]
        .join("\n")
    }
}

/// The tables a workgroup plays, one per lane.
pub const TABLES_PER_WORKGROUP: usize = 32;

/// How many years one dispatch plays before the tables come back.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Params {
    pub years: u32,
}

/// The year kernel on a batch of genomes: tables are played in place, as
/// many years at a time as asked.
pub struct Arena<'a> {
    gpu: &'a Gpu,
    pipeline: wgpu::ComputePipeline,
    genomes: wgpu::Buffer,
}

impl<'a> Arena<'a> {
    /// The kernel, compiled, over `genomes` (each of `Brain::GENOME`
    /// weights, in the CPU's layout).
    pub fn new(gpu: &'a Gpu, genomes: &[Vec<f32>]) -> Arena<'a> {
        let pipeline = gpu.pipeline("year", &shaders::year());
        let laid: Vec<f32> = genomes.iter().flat_map(|g| layout::laid(g)).collect();
        let genomes = gpu.upload("genomes", &laid);
        Arena {
            gpu,
            pipeline,
            genomes,
        }
    }

    /// `tables` after `years` more years each (fewer once a table is
    /// done).
    pub fn play(&self, tables: &[state::Table], years: u32) -> Vec<state::Table> {
        let buffer = self.gpu.output("tables", std::mem::size_of_val(tables) / 4);
        self.gpu
            .queue
            .write_buffer(&buffer, 0, bytemuck::cast_slice(tables));
        let params = self.gpu.upload("params", &[Params { years }]);
        self.gpu.dispatch(
            &self.pipeline,
            &[&self.genomes, &buffer, &params],
            tables.len().div_ceil(TABLES_PER_WORKGROUP) as u32,
        );
        self.gpu.download(&buffer)
    }
}

/// A question to the forward test kernel: a genome, a network (0 the
/// Intendance, 1 the Extérieur).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Question {
    pub genome: u32,
    pub net: u32,
}
