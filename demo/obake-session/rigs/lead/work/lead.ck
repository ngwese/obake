// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT
//
// Two-channel voice for the lead rig.
// outport 1 -> role melody, outport 2 -> role bed

SinOsc mel => dac.chan(0);
SinOsc bed => dac.chan(1);

0.15 => mel.gain;
0.08 => bed.gain;
220.0 => mel.freq;
110.0 => bed.freq;

while (true) {
    0.25::second => now;
    mel.freq() * 1.5 => float next;
    if (next > 880.0) 220.0 => next;
    next => mel.freq;
}
