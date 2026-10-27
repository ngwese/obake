// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT
//
// Pad voice for the space rig (role pad -> space.wet).

SinOsc s => dac.chan(0);
0.08 => s.gain;
165.0 => s.freq;

while (true) {
    8::second => now;
    if (s.freq() < 200.0) {
        247.5 => s.freq;
    } else {
        165.0 => s.freq;
    }
}
