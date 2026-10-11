set -eu
fail() { printf 'FAIL: %s\n' "$1" >&2; exit 1; }

. /etc/os-release
[ "$ID" = debian ] || fail "ID=$ID"
[ "$VERSION_CODENAME" = trixie ] || fail "VERSION_CODENAME=$VERSION_CODENAME"
printf 'os-release ok\n'

for cmd in gcc g++ clang clang++ cmake make ld curl wget python3 git rustup rustc cargo; do
  command -v "$cmd" >/dev/null || fail "missing $cmd"
done
gcc --version >/dev/null
g++ --version >/dev/null
clang --version >/dev/null
clang++ --version >/dev/null
cmake --version >/dev/null
make --version >/dev/null
ld --version >/dev/null
curl --version >/dev/null
wget --version >/dev/null
python3 --version >/dev/null
git --version >/dev/null
printf 'versions ok\n'

work=$(mktemp -d)
printf 'int main(void){return 0;}\n' > "$work/t.c"
printf 'int main(){return 0;}\n' > "$work/t.cpp"
gcc -o "$work/t-gcc" "$work/t.c"
g++ -o "$work/t-gxx" "$work/t.cpp"
clang -o "$work/t-clang" "$work/t.c"
clang++ -o "$work/t-clangxx" "$work/t.cpp"
"$work/t-gcc"
"$work/t-gxx"
"$work/t-clang"
"$work/t-clangxx"
printf 'compile ok\n'

if command -v sudo >/dev/null 2>&1; then
  fail "sudo is on PATH"
fi
printf 'sudo absent\n'

if touch /usr/obake-write-test 2>/dev/null; then
  fail "wrote under /usr"
fi
printf 'root read-only ok\n'

rustup show > "$work/rustup-show.txt"
grep -q 'stable' "$work/rustup-show.txt" || fail "rustup show missing stable"
grep -Eq 'default|active' "$work/rustup-show.txt" || fail "rustup show missing default"
rustc --version > "$work/rustc-version.txt"
grep -Eq 'rustc [0-9]+\.[0-9]+\.[0-9]+' "$work/rustc-version.txt" || fail "rustc version"
if grep -Eq 'nightly|beta' "$work/rustc-version.txt"; then
  fail "rustc is not stable"
fi
printf 'rust stable ok\n'
cat "$work/rustup-show.txt"
cat "$work/rustc-version.txt"

host_line=$(rustc -vV | sed -n 's/^host: //p')
arch=$(uname -m)
case "$host_line" in
  "$arch"-*) ;;
  *) fail "host triple $host_line does not match $arch" ;;
esac
printf 'host triple %s ok\n' "$host_line"

expected="${HOME}/.cargo"
[ "$CARGO_HOME" = "$expected" ] || fail "CARGO_HOME=$CARGO_HOME expected $expected"
fetch=$(mktemp -d)
mkdir -p "$fetch/src"
cat > "$fetch/Cargo.toml" << 'TOML'
[package]
name = "obake_builder_check"
version = "0.1.0"
edition = "2021"

[dependencies]
libc = "0.2"
TOML
printf 'fn main() {}\n' > "$fetch/src/main.rs"
cargo fetch --manifest-path "$fetch/Cargo.toml" >/dev/null
crate=$(find "$expected/registry" -type d -name 'libc-*' -print -quit)
[ -n "$crate" ] || fail "libc crate not under $expected/registry"
printf 'cargo cache %s ok\n' "$crate"
