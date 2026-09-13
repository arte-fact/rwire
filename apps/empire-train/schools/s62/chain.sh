#!/bin/bash
# After the running s62b (les trois + rien) writes its measures, the six other readings follow.
until grep -q "measures written\|Traceback" /home/artefact/Workspace/rwire/apps/empire-train/schools/s62/driver-300.out; do sleep 30; done
cd /home/artefact/Workspace/rwire/apps/empire-train/schools && python3 run-s62b.py told recall journal told+recall told+journal recall+journal > /home/artefact/Workspace/rwire/apps/empire-train/schools/s62/driver-300b.out 2>&1
echo chain-done >> /home/artefact/Workspace/rwire/apps/empire-train/schools/s62/driver-300b.out
