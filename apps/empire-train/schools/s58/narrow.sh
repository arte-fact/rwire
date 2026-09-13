#!/bin/bash
# h16, h8, h4 of s58 on the CPU, while the GPU plays the wide ones.
PROD="--against /home/artefact/Workspace/rwire/libs/empire-lib/brains/soldat.f32 --against /home/artefact/Workspace/rwire/libs/empire-lib/brains/batisseuse.f32 --against /home/artefact/Workspace/rwire/libs/empire-lib/brains/garnison.f32 --against /home/artefact/Workspace/rwire/libs/empire-lib/brains/boutiquiere.f32 --against /home/artefact/Workspace/rwire/libs/empire-lib/brains/fonceuse.f32 --against /home/artefact/Workspace/rwire/libs/empire-lib/brains/conquerante.f32 --against /home/artefact/Workspace/rwire/libs/empire-lib/brains/prudente.f32"
for H in 16 8 4; do
  RAYON_NUM_THREADS=10 /home/artefact/Workspace/rwire/target/release/empire-train --stage war $PROD --hall 0 --longest 100 --rank 40 --tables 6 --generations 300 --trials 8 --hidden $H --keep 25 --out "/home/artefact/Workspace/rwire/apps/empire-train/schools/s58/h$H-{n}-war.json" > /home/artefact/Workspace/rwire/apps/empire-train/schools/s58/h$H.log 2>&1
done
echo narrow-done
