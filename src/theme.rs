use crate::SETTINGS;
use skia_safe::{Color, Font, Paint, PaintStyle};
use std::{collections::HashMap, sync::OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GorlColor {
    WndBg,
    CtrlBg,
    CtrlFg,
    Highlight,
}

#[derive(Debug)]
pub struct FontCache {
    pub ui_default: skia_safe::Font,
    pub log_view: skia_safe::Font,
}

impl FontCache {
    // Shared font loading logic
    fn new(settings: &crate::settings::Settings, ctrl_ft_size: f32) -> Self {
        let font_mgr = skia_safe::FontMgr::new();

        let load_font = |name: &str, size: f32| -> skia_safe::Font {
            skia_safe::Font::new(
                font_mgr
                    .legacy_make_typeface(name, skia_safe::FontStyle::normal())
                    .unwrap_or_else(|| panic!("Failed to load font: {}", name)),
                size,
            )
        };

        FontCache {
            ui_default: load_font(settings.font.name.as_str(), ctrl_ft_size),
            log_view: load_font(settings.font.name.as_str(), settings.font.size as f32),
        }
    }
}

#[derive(Debug)]
pub struct ThemeCache {
    colors: HashMap<GorlColor, Color>,
    fonts: FontCache,
    pub ctrl_ft_size: f32,
}

static THEME_CACHE: OnceLock<ThemeCache> = OnceLock::new();
pub const CTRL_FT_SIZE: f32 = 16.0;

pub fn initialize_theme() {
    let settings = SETTINGS.read().expect("SETTINGS");

    let mut colors = HashMap::new();
    colors.insert(GorlColor::WndBg, Color::from_argb(255, 30, 30, 30));
    colors.insert(GorlColor::CtrlBg, Color::from_argb(255, 60, 60, 60));
    colors.insert(GorlColor::CtrlFg, Color::from_argb(255, 180, 180, 180));
    colors.insert(GorlColor::Highlight, Color::from_argb(255, 100, 149, 237)); // Cornflower Blue

    let cache = ThemeCache {
        colors,
        fonts: FontCache::new(&settings, CTRL_FT_SIZE),
        ctrl_ft_size: CTRL_FT_SIZE,
    };

    THEME_CACHE
        .set(cache)
        .expect("ThemeCache should be initialized once.");
}

fn get_cache() -> &'static ThemeCache {
    THEME_CACHE
        .get()
        .expect("ThemeCache not initialized. Call initialize_theme() first.")
}

pub fn get_color(color: GorlColor) -> Color {
    get_cache()
        .colors
        .get(&color)
        .copied()
        .unwrap_or_else(|| panic!("Theme color {:?} not found in cache", color))
}

pub fn solid(color: GorlColor) -> Paint {
    let mut p = Paint::default();
    p.set_color(get_color(color));
    p.set_style(PaintStyle::Fill);
    p
}

pub fn half_solid(color: GorlColor) -> Paint {
    let base_color = get_color(color);
    let mut p = Paint::default();

    let semi_transparent_color = base_color.with_a(128);

    p.set_color(semi_transparent_color);
    p.set_style(PaintStyle::Fill);
    p
}

pub fn ui_font() -> &'static Font {
    &get_cache().fonts.ui_default
}

pub fn log_view_font() -> &'static Font {
    &get_cache().fonts.log_view
}
