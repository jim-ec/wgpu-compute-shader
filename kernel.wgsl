@group(0) @binding(0)
var<storage, read_write> result: array<i32>;

@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    result[id.x] *= 2;
}
