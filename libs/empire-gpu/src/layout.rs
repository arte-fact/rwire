//! How a genome lies in the GPU's memory, read four weights at a time:
//! the first layer of each network input-major (the `hidden` weights of
//! an input, then the `hidden` biases), the second one row per output,
//! its `hidden` weights, its bias and three zeros to keep the rows
//! aligned. The kernels are compiled for one hidden width, a multiple
//! of 32 (they read it 32 neurons at a time).

use empire_lib::brain::{Net, Shape, A_IN, A_OUT, B_IN, B_OUT, DEADBAND};

const fn laid_len(inputs: usize, outputs: usize, hidden: usize) -> usize {
    (inputs + 1) * hidden + outputs * (hidden + 4)
}

/// Weights of the Intendance as laid out, where the Extérieur's begin.
pub const fn a_len(hidden: usize) -> usize {
    laid_len(A_IN, A_OUT, hidden)
}

/// A genome as laid out.
pub const fn genome(hidden: usize) -> usize {
    a_len(hidden) + laid_len(B_IN, B_OUT, hidden)
}

fn lay_net(w: &[f32], inputs: usize, hidden: usize, out: &mut [f32]) {
    let (w1, w2) = w.split_at((inputs + 1) * hidden);
    let (o1, o2) = out.split_at_mut((inputs + 1) * hidden);
    for i in 0..=inputs {
        for j in 0..hidden {
            o1[i * hidden + j] = w1[j * (inputs + 1) + i];
        }
    }
    for (o, row) in o2.chunks_exact_mut(hidden + 4).enumerate() {
        row[..hidden + 1].copy_from_slice(&w2[o * (hidden + 1)..(o + 1) * (hidden + 1)]);
        row[hidden + 1..].fill(0.0);
    }
}

/// `genome`, of `hidden` neurons, laid out for the GPU into `out`,
/// [`genome`]`(hidden)` long.
pub fn lay(g: &[f32], hidden: usize, out: &mut [f32]) {
    assert_eq!(g.len(), Shape::wide(hidden).genome());
    assert_eq!(out.len(), genome(hidden));
    let (a, b) = g.split_at(Net::len(A_IN, A_OUT, hidden));
    let (oa, ob) = out.split_at_mut(a_len(hidden));
    lay_net(a, A_IN, hidden, oa);
    lay_net(b, B_IN, hidden, ob);
}

/// A genome of `hidden` neurons laid out for the GPU.
pub fn laid(g: &[f32], hidden: usize) -> Vec<f32> {
    let mut out = vec![0.0; genome(hidden)];
    lay(g, hidden, &mut out);
    out
}

/// The constants the shaders are compiled with: the brain's widths, the
/// laid-out lengths in vec4, its deadband, and `ln 1.5` as the CPU's f32
/// has it (the army's efficiency is drawn on it).
pub fn header(hidden: usize) -> String {
    assert!(
        hidden > 0 && hidden.is_multiple_of(32),
        "a hidden layer of {hidden}: the kernels read 32 neurons at a time"
    );
    format!(
        "const HIDDEN: u32 = {hidden}u;\nconst A_IN: u32 = {A_IN}u;\nconst A_OUT: u32 = {A_OUT}u;\n\
         const B_IN: u32 = {B_IN}u;\nconst B_OUT: u32 = {B_OUT}u;\nconst A_LEN: u32 = {}u;\n\
         const GENOME: u32 = {}u;\nconst DEADBAND: f32 = {DEADBAND:?};\nconst LN_1_5: f32 = {:?};\n",
        a_len(hidden) / 4,
        genome(hidden) / 4,
        1.5f32.ln()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layout_is_vec4_aligned() {
        for hidden in [32, 64, 256] {
            assert_eq!(a_len(hidden) % 4, 0);
            assert_eq!(genome(hidden) % 4, 0);
        }
        assert_eq!(laid(&vec![0.5; Shape::NOW.genome()], 32).len(), genome(32));
        assert_eq!(
            laid(&vec![0.5; Shape::wide(64).genome()], 64).len(),
            genome(64)
        );
    }
}
