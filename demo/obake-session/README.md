# Example host profile and rigs

Material for `obake-session`. Copy onto a host, then start a session.

## Layout on the host

```
~/.config/obake-session/host.yaml
~/.config/obake-session/rigs/<name>.yaml
~/.local/share/obake-session/rigs/<name>/work/
```

## Install by hand

```bash
install -d ~/.config/obake-session/rigs
install -m 0644 host.yaml ~/.config/obake-session/host.yaml
install -m 0644 rigs/*.yaml ~/.config/obake-session/rigs/

for name in lead space tone; do
  dest=~/.local/share/obake-session/rigs/$name/work
  install -d "$dest"
  install -m 0644 work/$name/* "$dest/"
done
```

Or install a single rig document over GraphQL:

```bash
obake rig install --file rigs/lead.yaml --name lead
# then place work/lead/* into the rig work area
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
| `lead` | one `chuck` | Design example: two out roles, no links |
| `space` | two `chuck` + `rnbo-runner` | Role fan-in onto one input |
| `tone` | one `chuck` | Minimal stereo source for MixPre / Ravenna |
