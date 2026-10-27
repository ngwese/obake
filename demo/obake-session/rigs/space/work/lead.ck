// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT
//
// Melody voice for the space rig (role melody -> space.wet).

SinOsc s => dac.chan(0);
0.12 => s.gain;
330.0 => s.freq;

[0, 2, 3, 5, 7, 8, 10, 12] @=> int scale[];
0 => int i;

while (true) {
    330.0 * Math.pow(2.0, scale[i] / 12.0) => s.freq;
    (i + 1) % scale.size() => i;
    0.5::second => now;
}
