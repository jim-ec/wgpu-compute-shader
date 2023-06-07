@group(0) @binding(0)
var<storage, read_write> result: i32;

@compute @workgroup_size(1)
fn main() {
    result *= 2;
}
