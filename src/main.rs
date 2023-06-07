use std::{fs, num::NonZeroU64, sync::Arc};

use wgpu::*;

#[async_std::main]
async fn main() {
    let instance = Instance::new(InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&RequestAdapterOptions::default())
        .await
        .expect("failed to find an adapter");

    let (device, queue) = adapter
        .request_device(&DeviceDescriptor::default(), None)
        .await
        .expect("failed to request a device");

    let shader_module = device.create_shader_module(ShaderModuleDescriptor {
        label: None,
        source: ShaderSource::Wgsl(
            fs::read_to_string("kernel.wgsl")
                .expect("failed to read shader file")
                .into(),
        ),
    });

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: None,
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            count: None,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: Some(NonZeroU64::new(4).unwrap()),
            },
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &shader_module,
        entry_point: "main",
    });

    let storage_buffer = device.create_buffer(&BufferDescriptor {
        label: None,
        mapped_at_creation: true,
        size: std::mem::size_of::<i32>() as BufferAddress,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
    });

    {
        let input: i32 = 21;
        println!("Input: {input}");
        let mut mapping: BufferViewMut = storage_buffer.slice(..).get_mapped_range_mut();
        for (i, byte) in input.to_ne_bytes().into_iter().enumerate() {
            mapping[i] = byte;
        }
        drop(mapping);
        storage_buffer.unmap();
    }

    // We cannot map a storage buffer, thus we copy its contents into a readback buffer and map it instead.
    let readback_buffer = Arc::new(device.create_buffer(&BufferDescriptor {
        label: None,
        mapped_at_creation: false,
        size: 4,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
    }));

    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::Buffer(BufferBinding {
                buffer: &storage_buffer,
                offset: 0,
                size: None,
            }),
        }],
    });

    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

    {
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor { label: None });
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_pipeline(&pipeline);
        pass.dispatch_workgroups(1, 1, 1);
    }

    encoder.copy_buffer_to_buffer(&storage_buffer, 0, &readback_buffer, 0, 4);

    queue.submit(Some(encoder.finish()));

    device.poll(MaintainBase::Wait);

    readback_buffer
        .clone()
        .slice(..)
        .map_async(MapMode::Read, move |result| {
            result.expect("failed to map storage buffer");
            let contents = readback_buffer.slice(..).get_mapped_range();
            let readback = contents
                .chunks_exact(std::mem::size_of::<i32>())
                .map(|bytes| i32::from_ne_bytes(bytes.try_into().unwrap()))
                .next()
                .unwrap();
            println!("Output: {readback}");
        })
}
