//! The GPU's forward against `Net::forward`, on random genomes and sights.

use empire_gpu::{layout, shaders, Gpu, Question};
use empire_lib::brain::{Brain, A_IN, A_OUT, B_IN, B_OUT};
use empire_lib::random::{random, seed};

fn noise(n: usize, scale: f32) -> Vec<f32> {
    (0..n)
        .map(|_| (random(0, 20_001) - 10_000) as f32 / 10_000.0 * scale)
        .collect()
}

#[test]
fn the_gpu_answers_as_the_cpu() {
    let Some(gpu) = Gpu::open() else {
        eprintln!("no GPU: skipped");
        return;
    };
    seed(3);
    let scales = Brain::scales();
    let genomes: Vec<Vec<f32>> = (0..4)
        .map(|_| {
            noise(Brain::GENOME, 1.0)
                .iter()
                .zip(&scales)
                .map(|(x, s)| x * s * 2.0)
                .collect()
        })
        .collect();
    let brains: Vec<Brain> = genomes
        .iter()
        .map(|g| Brain::from_genome(g, true))
        .collect();
    let questions: Vec<Question> = (0..40)
        .map(|i| Question {
            genome: i % 4,
            net: (i / 4) % 2,
        })
        .collect();
    let sights: Vec<Vec<f32>> = questions
        .iter()
        .map(|q| {
            let mut s = noise(if q.net == 0 { A_IN } else { B_IN }, 1.5);
            s.resize(B_IN, 0.0);
            s
        })
        .collect();

    let flat: Vec<f32> = genomes.iter().flat_map(|g| layout::laid(g)).collect();
    let pipeline = gpu.pipeline("forward test", &shaders::forward_test());
    let genomes_buf = gpu.upload("genomes", &flat);
    let questions_buf = gpu.upload("questions", &questions);
    let sights_buf = gpu.upload("sights", &sights.concat());
    let answers_buf = gpu.output("answers", questions.len() * B_OUT);
    gpu.dispatch(
        &pipeline,
        &[&genomes_buf, &questions_buf, &sights_buf, &answers_buf],
        questions.len().div_ceil(32) as u32,
    );
    let answers: Vec<f32> = gpu.download(&answers_buf);

    let mut worst = 0.0f32;
    for (i, q) in questions.iter().enumerate() {
        let b = &brains[q.genome as usize];
        let (expected, width) = if q.net == 0 {
            (b.intendance.forward(&sights[i][..A_IN]), A_OUT)
        } else {
            (b.exterieur.forward(&sights[i][..B_IN]), B_OUT)
        };
        let got = &answers[i * B_OUT..i * B_OUT + width];
        for (o, (&e, &g)) in expected.iter().zip(got).enumerate() {
            let err = (e - g).abs();
            worst = worst.max(err);
            assert!(err < 1e-4, "question {i} answer {o}: cpu {e} gpu {g}");
        }
    }
    eprintln!("{}: worst error {worst:e}", gpu.name);
}
