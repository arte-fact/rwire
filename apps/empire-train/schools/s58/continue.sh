#!/bin/bash
# s58, 300 generations more: h32 as the control, then h256 whose best trials were still climbing.
PROD="--against /home/artefact/Workspace/rwire/libs/empire-lib/brains/soldat.f32 --against /home/artefact/Workspace/rwire/libs/empire-lib/brains/batisseuse.f32 --against /home/artefact/Workspace/rwire/libs/empire-lib/brains/garnison.f32 --against /home/artefact/Workspace/rwire/libs/empire-lib/brains/boutiquiere.f32 --against /home/artefact/Workspace/rwire/libs/empire-lib/brains/fonceuse.f32 --against /home/artefact/Workspace/rwire/libs/empire-lib/brains/conquerante.f32 --against /home/artefact/Workspace/rwire/libs/empire-lib/brains/prudente.f32"
for H in 32 256; do
  /home/artefact/Workspace/rwire/target/release/empire-train --stage war $PROD --hall 0 --longest 100 --rank 40 --tables 6 --generations 300 --trials 8 --hidden $H --keep 25 --gpu --from "/home/artefact/Workspace/rwire/apps/empire-train/schools/s58/h$H-{n}-war.json" --out "/home/artefact/Workspace/rwire/apps/empire-train/schools/s58/h$H-{n}-war.json" > /home/artefact/Workspace/rwire/apps/empire-train/schools/s58/h$H-600.log 2>&1
done
echo continue-done
