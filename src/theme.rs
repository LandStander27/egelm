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

		let surface = background.lighten(0.05);
		let surface_variant = background.lighten(0.10);
		let border = background.lighten(0.12);

		let background_color = background.color();
		let surface_color = surface.color();
		let surface_variant_color = surface_variant.color();
		let border_color = border.color();
		let text = self.text.color();
		let text_muted = self.text_muted.color();

		// If primary is light (lightness > 0.55), darken it for button fills
		// so light text (#f9d5c7) always maintains high contrast (>4.5:1)
		let (_, _, primary_lightness) = self.primary.rgb_to_hsl();
		let active_bg = if primary_lightness > 0.50 {
			self.primary.darken(primary_lightness - 0.35)
		} else {
			self.primary
		};

		// General
		visuals.panel_fill = background_color;
		visuals.window_fill = background_color;
		visuals.window_stroke = Stroke::new(1.0, border_color);
		visuals.faint_bg_color = background.lighten(0.02).color();
		visuals.extreme_bg_color = background.darken(0.03).color();

		// Selection
		visuals.selection.bg_fill = active_bg.color();
		visuals.selection.stroke = Stroke::new(2.0, text);

		// Non-interactive
		visuals.widgets.noninteractive = WidgetVisuals {
			bg_fill: background_color,
			weak_bg_fill: background_color,
			bg_stroke: Stroke::new(1.0, border_color),
			fg_stroke: Stroke::new(1.0, text_muted),
			corner_radius: 6.into(),
			expansion: 0.0,
		};

		// Normal widgets
		visuals.widgets.inactive = WidgetVisuals {
			bg_fill: surface_color,
			weak_bg_fill: surface_color,
			bg_stroke: Stroke::NONE,
			fg_stroke: Stroke::new(1.0, text),
			corner_radius: 6.into(),
			expansion: 0.0,
		};

		// Hovered widgets
		visuals.widgets.hovered = WidgetVisuals {
			bg_fill: surface_variant_color,
			weak_bg_fill: surface_variant_color,
			bg_stroke: Stroke::new(1.0, border.lighten(0.05).color()),
			fg_stroke: Stroke::new(1.0, text),
			corner_radius: 6.into(),
			expansion: 1.0,
		};

		// Pressed / Toggled widgets
		visuals.widgets.active = WidgetVisuals {
			bg_fill: active_bg.color(),
			weak_bg_fill: active_bg.color(),
			bg_stroke: Stroke::NONE,
			fg_stroke: Stroke::new(1.0, text),
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
