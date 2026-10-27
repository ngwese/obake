# Example host profile and rigs

Material for `obake-session`. Copy onto a host, then start a session.

## Layout on the host

```
~/.config/obake-session/host.yaml
~/.config/obake-session/rigs/<name>/rig.yaml
~/.config/obake-session/rigs/<name>/work/…
~/.local/share/obake-session/sessions/<id>/work/   # seeded on fresh start
```

Each demo rig is a directory package: `rig.yaml` plus an optional
`work/` template. A fresh session copies that template into the new
session work directory (mounted at `/work` inside shapes).

## Install by hand

```bash
install -d ~/.config/obake-session/rigs
install -m 0644 host.yaml ~/.config/obake-session/host.yaml
cp -a rigs/lead rigs/space rigs/tone ~/.config/obake-session/rigs/
```

Or scaffold and install from the CLI:

```bash
obake rig init myrig
# edit myrig/rig.yaml and myrig/work/…
obake rig create myrig --path ./myrig
```

## Start

```bash
obake start lead --mode fresh
# route lead.melody / lead.bed to hardware or Ravenna:
obake links set --system '[{"from":{"instance":"lead","role":"melody"},"to":{"alias":"mixpre","port":"playback_1"}}]'
obake stop
```

System links are session state; they are not written into the rig YAML.

## Rigs

| Name | Shapes | What it demonstrates |
|---|---|---|
| `lead` | one `chuck-alpine` | Design example: two out roles, no links |
| `space` | two `chuck-alpine` + `rnbo-runner` | Role fan-in onto one input |
| `tone` | one `chuck-alpine` | Minimal stereo source for MixPre / Ravenna |
