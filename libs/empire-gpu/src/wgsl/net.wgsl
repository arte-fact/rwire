// The two networks of a brain, run by the lane on its own table: the
// 32 hidden neurons in eight vec4 of registers, the weights read four
// at a time (see `layout`).

// The sight the lane builds for its table, and the answer it gets back.
var<private> x: array<f32, B_IN>;
var<private> y: array<f32, B_OUT>;

fn sigmoid(s: f32) -> f32 {
    return 1.0 / (1.0 + exp(-s));
}

// The network whose weights start at `base` (in vec4) in `genomes`, of
// `inputs` entries and `outputs` answers, read on `x[0..inputs]` into
// `y[0..outputs]`.
fn forward(base: u32, inputs: u32, outputs: u32) {
    var a0 = vec4(0.0);
    var a1 = vec4(0.0);
    var a2 = vec4(0.0);
    var a3 = vec4(0.0);
    var a4 = vec4(0.0);
    var a5 = vec4(0.0);
    var a6 = vec4(0.0);
    var a7 = vec4(0.0);
    for (var i = 0u; i < inputs; i++) {
        let xi = x[i];
        let w = base + i * 8u;
        a0 += xi * genomes[w];
        a1 += xi * genomes[w + 1u];
        a2 += xi * genomes[w + 2u];
        a3 += xi * genomes[w + 3u];
        a4 += xi * genomes[w + 4u];
        a5 += xi * genomes[w + 5u];
        a6 += xi * genomes[w + 6u];
        a7 += xi * genomes[w + 7u];
    }
    let b = base + inputs * 8u;
    let h0 = tanh(genomes[b] + a0);
    let h1 = tanh(genomes[b + 1u] + a1);
    let h2 = tanh(genomes[b + 2u] + a2);
    let h3 = tanh(genomes[b + 3u] + a3);
    let h4 = tanh(genomes[b + 4u] + a4);
    let h5 = tanh(genomes[b + 5u] + a5);
    let h6 = tanh(genomes[b + 6u] + a6);
    let h7 = tanh(genomes[b + 7u] + a7);
    let w2 = b + 8u;
    for (var o = 0u; o < outputs; o++) {
        let r = w2 + o * 9u;
        let s = genomes[r + 8u].x
            + ((dot(genomes[r], h0) + dot(genomes[r + 1u], h1))
                + (dot(genomes[r + 2u], h2) + dot(genomes[r + 3u], h3)))
            + ((dot(genomes[r + 4u], h4) + dot(genomes[r + 5u], h5))
                + (dot(genomes[r + 6u], h6) + dot(genomes[r + 7u], h7)));
        y[o] = sigmoid(s);
    }
}

// The Intendance of genome `g`.
fn forward_a(g: u32) {
    forward(g * GENOME, A_IN, A_OUT);
}

// The Extérieur of genome `g`.
fn forward_b(g: u32) {
    forward(g * GENOME + A_LEN, B_IN, B_OUT);
}
