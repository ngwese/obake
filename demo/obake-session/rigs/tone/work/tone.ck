// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT
//
// Steady stereo tone for system-link checks.
// outport 1 -> left, outport 2 -> right

SinOsc L => dac.chan(0);
SinOsc R => dac.chan(1);
0.1 => L.gain => R.gain;
440.0 => L.freq;
441.0 => R.freq;

while (true) {
    1::second => now;
}
