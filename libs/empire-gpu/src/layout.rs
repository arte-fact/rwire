//! How a genome lies in the GPU's memory, read four weights at a time:
//! the first layer of each network input-major (the 32 weights of an
//! input, then the 32 biases), the second one row per output, its 32
//! weights, its bias and three zeros to keep the rows aligned.

use empire_lib::brain::{Brain, Net, A_IN, A_OUT, B_IN, B_OUT, DEADBAND, HIDDEN};

/// A row of the second layer.
const ROW: usize = HIDDEN + 4;

const fn laid_len(inputs: usize, outputs: usize) -> usize {
    (inputs + 1) * HIDDEN + outputs * ROW
}

/// Weights of the Intendance as laid out, where the Extérieur's begin.
pub const A_LEN: usize = laid_len(A_IN, A_OUT);

/// A genome as laid out.
pub const GENOME: usize = A_LEN + laid_len(B_IN, B_OUT);

fn lay_net(w: &[f32], inputs: usize, outputs: usize, out: &mut Vec<f32>) {
    let (w1, w2) = w.split_at((inputs + 1) * HIDDEN);
    for i in 0..=inputs {
        for j in 0..HIDDEN {
            out.push(w1[j * (inputs + 1) + i]);
        }
    }
    for o in 0..outputs {
        out.extend_from_slice(&w2[o * (HIDDEN + 1)..(o + 1) * (HIDDEN + 1)]);
        out.extend_from_slice(&[0.0; ROW - HIDDEN - 1]);
    }
}

/// A genome laid out for the GPU.
pub fn laid(genome: &[f32]) -> Vec<f32> {
    assert_eq!(genome.len(), Brain::GENOME);
    let mut out = Vec::with_capacity(GENOME);
    let (a, b) = genome.split_at(Net::len(A_IN, A_OUT));
    lay_net(a, A_IN, A_OUT, &mut out);
    lay_net(b, B_IN, B_OUT, &mut out);
    out
}

/// The constants the shaders are compiled with: the brain's widths, the
/// laid-out lengths in vec4, its deadband, and `ln 1.5` as the CPU's f32
/// has it (the army's efficiency is drawn on it).
pub fn header() -> String {
    format!(
        "const HIDDEN: u32 = {HIDDEN}u;\nconst A_IN: u32 = {A_IN}u;\nconst A_OUT: u32 = {A_OUT}u;\n\
         const B_IN: u32 = {B_IN}u;\nconst B_OUT: u32 = {B_OUT}u;\nconst A_LEN: u32 = {}u;\n\
         const GENOME: u32 = {}u;\nconst DEADBAND: f32 = {DEADBAND:?};\nconst LN_1_5: f32 = {:?};\n",
        A_LEN / 4,
        GENOME / 4,
        1.5f32.ln()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layout_is_vec4_aligned() {
        assert_eq!(A_LEN % 4, 0);
        assert_eq!(GENOME % 4, 0);
        assert_eq!(laid(&vec![0.5; Brain::GENOME]).len(), GENOME);
    }
}
