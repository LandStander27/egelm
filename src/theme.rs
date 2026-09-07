use egui::{Color32, Stroke, Visuals, style::WidgetVisuals};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Theme {
	pub colors: Colors,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Colors {
	pub background: Color,
	pub text: Color,
	pub text_muted: Color,
	pub text_disabled: Color,
	pub primary: Color,
	pub primary_text: Color,
	pub success: Color,
	pub warning: Color,
	pub error: Color,
}

impl Colors {
	pub(crate) fn to_visuals(&self) -> Visuals {
		let mut visuals = Visuals::dark();

		let background = self.background;

		// Return to a subtle, modern flat look without excessive lighting jumps
		let surface = background.lighten(0.05);
		let surface_variant = background.lighten(0.10);
		let border = background.lighten(0.12);

		let background_color = background.color();
		let surface_color = surface.color();
		let surface_variant_color = surface_variant.color();
		let border_color = border.color();
		let selection_color = self.primary.color();

		let text = self.text.color();
		let primary = self.primary.color();
		let primary_text = self.primary_text.color();
		let text_muted = self.text_muted.color();

		// General
		visuals.panel_fill = background_color;
		visuals.window_fill = background_color;
		visuals.window_stroke = Stroke::new(1.0, border_color);

		// Ensure different background roles are mapped uniquely so widgets don't accidentally blend into frames
		visuals.faint_bg_color = background.lighten(0.02).color();
		visuals.extreme_bg_color = background.darken(0.03).color();

		// This sets standard text color (RichText::new) to our regular text mapping
		visuals.override_text_color = Some(text);

		// Selection
		visuals.selection.bg_fill = selection_color;
		// Thicken the geometric selection stroke from 1.0 to 2.0 so toggle switch thumbs, checkboxes,
		// and active rings clearly pop against their backgrounds! (Egui's demo toggle switch draws
		// its interior with `selection.bg_fill` identically to the track behind it, relying purely on THIS
		// stroke to separate the thumb from the track).
		visuals.selection.stroke = Stroke::new(2.0, primary_text);

		// Non-interactive
		// NOTE: egui derives `visuals.weak_text_color()` (used for RichText::weak()) from `noninteractive.fg_stroke.color`!
		visuals.widgets.noninteractive = WidgetVisuals {
			bg_fill: background_color,
			weak_bg_fill: background_color,
			bg_stroke: Stroke::new(1.0, border_color),
			fg_stroke: Stroke::new(1.0, text_muted), // Provides the explicit muted text color
			corner_radius: 6.into(),
			expansion: 0.0,
		};

		// Normal widgets
		visuals.widgets.inactive = WidgetVisuals {
			bg_fill: surface_color,
			weak_bg_fill: surface_color,
			bg_stroke: Stroke::NONE, // Revert to borderless flat look
			fg_stroke: Stroke::new(1.0, text),
			corner_radius: 6.into(),
			expansion: 0.0,
		};

		// Hovered widgets
		visuals.widgets.hovered = WidgetVisuals {
			bg_fill: surface_variant_color,
			weak_bg_fill: surface_variant_color,
			bg_stroke: Stroke::new(1.0, border.lighten(0.05).color()), // Only show border on hover
			fg_stroke: Stroke::new(1.0, text),
			corner_radius: 6.into(),
			expansion: 1.0,
		};

		// Pressed widgets
		// NOTE: egui uniquely maps `visuals.strong_text_color()` directly to `visuals.widgets.active.fg_stroke.color`!
		// Because we're in a dark theme, if we set this to `primary_text` (which is often dark to contrast with bright buttons),
		// it makes `RichText::strong()` effectively invisible on the dark window panel!
		// To fix this, we map it to a brightened version of the normal text color so strong text always pops.
		visuals.widgets.active = WidgetVisuals {
			bg_fill: primary,
			weak_bg_fill: primary,
			bg_stroke: Stroke::NONE,
			fg_stroke: Stroke::new(1.0, self.text.lighten(0.20).color()),
			corner_radius: 6.into(),
			expansion: 0.0,
		};

		// Open widgets
		visuals.widgets.open = WidgetVisuals {
			bg_fill: surface_color,
			weak_bg_fill: surface_color,
			bg_stroke: Stroke::new(1.0, border_color),
			fg_stroke: Stroke::new(1.0, text),
			corner_radius: 6.into(),
			expansion: 0.0,
		};

		// Misc
		visuals.hyperlink_color = self.primary.lighten(0.15).color();
		visuals.warn_fg_color = self.warning.color();
		visuals.error_fg_color = self.error.color();

		visuals
	}
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Color {
	pub(crate) r: u8,
	pub(crate) g: u8,
	pub(crate) b: u8,
}

impl Color {
	pub(crate) fn color(&self) -> Color32 {
		Color32::from_rgb(self.r, self.g, self.b)
	}

	pub(crate) fn rgb_to_hsl(&self) -> (f32, f32, f32) {
		let r = self.r as f32 / 255.0;
		let g = self.g as f32 / 255.0;
		let b = self.b as f32 / 255.0;

		let max = r.max(g).max(b);
		let min = r.min(g).min(b);

		let lightness = (max + min) / 2.0;

		if max == min {
			return (0.0, 0.0, lightness);
		}

		let delta = max - min;

		let saturation = if lightness > 0.5 {
			delta / (2.0 - max - min)
		} else {
			delta / (max + min)
		};

		let hue = if max == r {
			((g - b) / delta) % 6.0
		} else if max == g {
			(b - r) / delta + 2.0
		} else {
			(r - g) / delta + 4.0
		};

		let hue = hue / 6.0;
		let hue = if hue < 0.0 { hue + 1.0 } else { hue };

		(hue, saturation, lightness)
	}

	pub(crate) fn hsl_to_rgb(h: f32, s: f32, l: f32) -> Self {
		if s == 0.0 {
			let value = (l * 255.0).round() as u8;
			return Self { r: value, g: value, b: value };
		}

		let q = if l < 0.5 {
			l * (1.0 + s)
		} else {
			l + s - l * s
		};

		let p = 2.0 * l - q;

		fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
			if t < 0.0 {
				t += 1.0;
			}
			if t > 1.0 {
				t -= 1.0;
			}
			if t < 1.0 / 6.0 {
				p + (q - p) * 6.0 * t
			} else if t < 1.0 / 2.0 {
				q
			} else if t < 2.0 / 3.0 {
				p + (q - p) * (2.0 / 3.0 - t) * 6.0
			} else {
				p
			}
		}

		let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
		let g = hue_to_rgb(p, q, h);
		let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

		Self {
			r: (r * 255.0).round() as u8,
			g: (g * 255.0).round() as u8,
			b: (b * 255.0).round() as u8,
		}
	}

	pub(crate) fn lighten(&self, amount: f32) -> Self {
		let (h, s, l) = self.rgb_to_hsl();
		Self::hsl_to_rgb(h, s, (l + amount).clamp(0.0, 1.0))
	}

	pub(crate) fn darken(&self, amount: f32) -> Self {
		self.lighten(-amount)
	}
}

impl Serialize for Color {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let value = format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b);
		serializer.serialize_str(&value)
	}
}

impl<'de> Deserialize<'de> for Color {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let value = String::deserialize(deserializer)?;
		let value = value
			.strip_prefix('#')
			.ok_or_else(|| serde::de::Error::custom("expected '#' prefix"))?;

		if value.len() != 6 {
			return Err(serde::de::Error::custom("expected 6 hexadecimal characters"));
		}

		let r = u8::from_str_radix(&value[0..2], 16).map_err(serde::de::Error::custom)?;
		let g = u8::from_str_radix(&value[2..4], 16).map_err(serde::de::Error::custom)?;
		let b = u8::from_str_radix(&value[4..6], 16).map_err(serde::de::Error::custom)?;

		Ok(Self { r, g, b })
	}
}

pub(crate) fn load_theme() -> Option<Theme> {
	let path = dirs::config_dir()?.join("egelm").join("colors.toml");
	if !path.exists() {
		return None;
	}

	let s = std::fs::read_to_string(&path)
		.inspect_err(|e| tracing::error!("could not read `{}`: {e}", path.display()))
		.ok()?;

	let theme: Theme = toml::from_str(&s)
		.inspect_err(|e| println!("could not parse `{}`: {e}", path.display()))
		.ok()?;

	Some(theme)
}

pub(crate) struct ThemeWatcher {
	_watcher: RecommendedWatcher,
}

impl ThemeWatcher {
	pub(crate) fn new(ctx: &egui::Context) -> Option<Self> {
		{
			if let Some(theme) = load_theme() {
				ctx.set_visuals(theme.colors.to_visuals());
				ctx.request_repaint();
			}
		}

		let mut watcher = notify::recommended_watcher({
			let ctx = ctx.clone();
			move |res: notify::Result<notify::Event>| {
				let event = match res {
					Ok(event) => event,
					Err(e) => {
						tracing::error!("{e}");
						return;
					}
				};

				if event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove() {
					if let Some(theme) = load_theme() {
						ctx.set_visuals(theme.colors.to_visuals());
					} else {
						ctx.set_visuals(egui::Visuals::default());
					}

					ctx.request_repaint();
				}
			}
		})
		.inspect_err(|e| tracing::error!("{e}"))
		.ok()?;

		let config_dir = dirs::config_dir()
			.or_else(|| {
				tracing::error!("could not find user's config directory");
				None
			})?
			.join("egelm")
			.join("colors.toml");
		let config_dir = config_dir.canonicalize().unwrap_or(config_dir);
		watcher
			.watch(&config_dir, RecursiveMode::NonRecursive)
			.inspect_err(|e| tracing::error!("{e}"))
			.ok()?;

		tracing::info!("watching `{}`", config_dir.display());
		Some(Self { _watcher: watcher })
	}
}
