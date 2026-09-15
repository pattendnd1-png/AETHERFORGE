# ForgeClean v1.0.4 NetworkCard-style Orbital Monitor Design

ForgeClean keeps the existing orbital worker as the single synchronization authority. The GUI adds a deep-purple DragonGlass live monitor modeled on the desktop Network Monitor / NetworkCard visual language: a scrolling two-trace history graph, upload/restore rate readouts, current migration phase/project/file, GitHub storage state, queue/parity state, and controls. No pie chart is used.

The one-second GUI refresh reads only the orbital status file, the tail of the live migration diagnostic, and `/proc/net/dev`. It must never invoke `git`, `gh`, hashing, discovery, or uploads from the refresh loop. Expensive work remains on the existing 30-second fast orbit / 5-minute full orbit.
