//! Setup cfg aliases for the crate

use cfg_aliases::cfg_aliases;

fn main() {
	cfg_aliases! {
		wasm: { target_arch = "wasm32" },
		android: { all(target_os = "android", feature = "android") },
		linux: { target_os = "linux" },
		x11: { all(linux, feature = "x11") },
		wayland: { all(linux, feature = "wayland") },
		glow: { feature = "glow" },
		wgpu: { feature = "wgpu" },
		ctrlc: { all(feature = "ctrlc", not(android)) },
	}
}
