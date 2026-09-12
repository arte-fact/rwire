// One lane per question: which genome, which network, on which sight —
// the answer written out for the CPU to compare. The shared buffers are
// fed straight, the table state stays out of it.

struct Question {
    genome: u32,
    // 0 for the Intendance, 1 for the Extérieur.
    net: u32,
}

@group(0) @binding(0) var<storage, read> genomes: array<vec4<f32>>;
@group(0) @binding(1) var<storage, read> questions: array<Question>;
@group(0) @binding(2) var<storage, read> sights: array<f32>;
@group(0) @binding(3) var<storage, read_write> answers: array<f32>;

@compute @workgroup_size(32)
fn main(@builtin(workgroup_id) wg: vec3<u32>, @builtin(local_invocation_id) local: vec3<u32>) {
    let n = wg.x * 32u + local.x;
    if (n >= arrayLength(&questions)) {
        return;
    }
    let q = questions[n];
    for (var i = 0u; i < B_IN; i++) {
        x[i] = sights[n * B_IN + i];
    }
    if (q.net == 0u) {
        forward_a(q.genome);
    } else {
        forward_b(q.genome);
    }
    for (var o = 0u; o < B_OUT; o++) {
        answers[n * B_OUT + o] = y[o];
    }
}
