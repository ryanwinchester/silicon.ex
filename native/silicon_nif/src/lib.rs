//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

use image::{DynamicImage, EncodableLayout};
use rustler::{
    lazy_static, Binary, Env, NewBinary, NifResult, NifStruct, NifTaggedEnum,
    ResourceArc, Term, NifUntaggedEnum,
};
use std::error::Error;
use std::io::Cursor;
use syntect::highlighting::{Theme, ThemeSet};

use silicon::assets::HighlightingAssets;
use silicon::formatter::ImageFormatterBuilder;
use silicon::utils::ShadowAdder;
use syntect::easy::HighlightLines;
use syntect::util::LinesWithEndings;

lazy_static::lazy_static! {
    static ref HIGHLIGHTING_ASSETS: HighlightingAssets = HighlightingAssets::new();
}

#[derive(Debug, NifStruct)]
#[module = "Silicon.RGBA"]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl From<Rgba> for image::Rgba<u8> {
    fn from(val: Rgba) -> Self {
        let Rgba { r, g, b, a } = val;
        image::Rgba::<u8>([r, g, b, a])
    }
}

#[derive(Debug, NifTaggedEnum)]
enum Background {
    Solid(Rgba),
    // Image(RgbaImage), // TODO
}

#[derive(Debug, NifStruct)]
#[module = "Silicon.Options.Shadow"]
struct ShadowOptions {
    background: Option<Background>,
    shadow_color: Option<Rgba>,
    blur_radius: Option<f32>,
    pad_horiz: Option<u32>,
    pad_vert: Option<u32>,
    offset_x: Option<i32>,
    offset_y: Option<i32>,
}

#[derive(Debug, NifStruct)]
#[module = "Silicon.Options.Image"]
struct ImageOptions {
    /// Pad between lines
    line_pad: Option<u32>,
    /// Show line number
    line_number: Option<bool>,
    /// Font of english character, should be mono space font
    /// Silicon docs say it will use Hack font (size: 26.0) by default
    font: Option<Vec<(String, f32)>>,
    /// Highlight lines
    highlight_lines: Option<Vec<u32>>,
    /// Whether show the window controls
    window_controls: Option<bool>,
    /// Window title
    window_title: Option<String>,
    /// Whether round the corner of the image
    round_corner: Option<bool>,
    /// Shadow adder,
    shadow_adder: Option<ShadowOptions>,
    /// Tab width
    tab_width: Option<u8>,
    /// Line Offset
    line_offset: Option<u32>,
}

#[derive(NifUntaggedEnum)]
enum ThemeEnum {
    Name(String),
    Resource(ResourceArc<ThemeSetResource>),
}

#[derive(NifStruct)]
#[module = "Silicon.Options.Format"]
struct FormatOptions {
    lang: String,
    theme: ThemeEnum,
    theme_set: Option<ResourceArc<ThemeSetResource>>,
    image_options: Option<crate::ImageOptions>,
}

struct ThemeSetResource {
    themes: Theme,
}

struct Wrapper<T>(T);

impl<T> Wrapper<T> {
    fn apply<U>(self, value: Option<U>, f: impl FnOnce(T, U) -> T) -> Self {
        if let Some(value) = value {
            Wrapper(f(self.0, value))
        } else {
            self
        }
    }

    fn flat_apply<U>(self, value: Option<U>, f: impl FnOnce(T, Option<U>) -> T) -> Self {
        self.apply(value, |obj, value| f(obj, Some(value)))
    }
}

fn do_image(opts: ImageOptions) -> ImageFormatterBuilder<String> {
    let Wrapper(format_builder) = Wrapper(ImageFormatterBuilder::<String>::new())
        .apply(opts.line_pad, ImageFormatterBuilder::line_pad)
        .apply(opts.line_number, ImageFormatterBuilder::line_number)
        .apply(opts.font, ImageFormatterBuilder::font)
        .apply(opts.highlight_lines, ImageFormatterBuilder::highlight_lines)
        .apply(opts.window_controls, ImageFormatterBuilder::window_controls)
        .flat_apply(opts.window_title, ImageFormatterBuilder::window_title)
        .apply(opts.round_corner, ImageFormatterBuilder::round_corner)
        .apply(opts.tab_width, ImageFormatterBuilder::tab_width)
        .apply(opts.line_offset, ImageFormatterBuilder::line_offset)
        .apply(opts.shadow_adder, do_shadow);
    format_builder
}
fn do_shadow<T: AsRef<str> + Default>(
    format_builder: ImageFormatterBuilder<T>,
    opts: ShadowOptions,
) -> ImageFormatterBuilder<T> {
    let Wrapper(shadow_builder) = Wrapper(ShadowAdder::default())
        .apply(opts.blur_radius, ShadowAdder::blur_radius)
        .apply(opts.pad_horiz, ShadowAdder::pad_horiz)
        .apply(opts.pad_vert, ShadowAdder::pad_vert)
        .apply(opts.offset_x, ShadowAdder::offset_x)
        .apply(opts.offset_y, ShadowAdder::offset_y)
        .apply(opts.shadow_color, |shadow_builder, value| {
            shadow_builder.shadow_color(value.into())
        })
        .apply(opts.background, |shadow_builder, background| {
            let bg = match background {
                Background::Solid(rgba) => silicon::utils::Background::Solid(rgba.into()),
            };
            shadow_builder.background(bg)
        });

    format_builder.shadow_adder(shadow_builder)
}

fn format(
    _env: Env<'_>,
    code: String,
    options: FormatOptions,
) -> Result<DynamicImage, Box<dyn Error>> {
    let theme = match options.theme {
        ThemeEnum::Name(name) => HIGHLIGHTING_ASSETS
            .theme_set
            .themes
            .get(name.as_str())
            .ok_or::<Box<dyn Error>>("Unknown theme".into())?,
        ThemeEnum::Resource(ref resource) => &resource.themes,
    };

    let syntax = HIGHLIGHTING_ASSETS
        .syntax_set
        .find_syntax_by_token(options.lang.as_str())
        .ok_or::<Box<dyn Error>>("Unknown lang".into())?;

    let mut h = HighlightLines::new(syntax, theme);
    let highlight = LinesWithEndings::from(code.as_str())
        .map(|line| h.highlight_line(line, &HIGHLIGHTING_ASSETS.syntax_set))
        .collect::<Result<Vec<_>, _>>()?;

    let image_builder = match options.image_options {
        Some(i_options) => do_image(i_options),
        None => ImageFormatterBuilder::new(),
    };

    let mut formatter = image_builder.build()?;

    let image = formatter.format(&highlight, &theme);

    Ok(image)
}

fn to_rustler_error(err: impl Into<Box<dyn Error>>) -> rustler::Error {
    rustler::Error::Term(Box::new(err.into().to_string()))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn load_theme(theme_data: Binary) -> NifResult<ResourceArc<ThemeSetResource>> {
    let theme_set = ThemeSet::load_from_reader(&mut Cursor::new(theme_data.as_bytes()))
        .map_err(to_rustler_error)?;
    Ok(ResourceArc::new(ThemeSetResource { themes: theme_set }))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_format_png(env: Env<'_>, code: String, options: FormatOptions) -> NifResult<Binary<'_>> {
    let mut bytes: Vec<u8> = Vec::new();

    format(env, code, options)
        .and_then(|image| {
            image.write_to(&mut Cursor::new(&mut bytes), image::ImageOutputFormat::Png)?;
            let mut out_binary = NewBinary::new(env, bytes.len());
            out_binary.as_mut_slice().copy_from_slice(bytes.as_bytes());

            Ok(out_binary.into())
        })
        .map_err(to_rustler_error)
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_format_rgba8(env: Env<'_>, code: String, options: FormatOptions) -> NifResult<Binary<'_>> {
    format(env, code, options)
        .and_then(|image| {
            let rgba8 = image
                .as_rgba8()
                .ok_or::<Box<dyn Error>>("Underlying library returned non-rgba image".into())?;
            let image_slice = rgba8.as_bytes();
            let mut out_binary = NewBinary::new(env, image_slice.len());
            out_binary.as_mut_slice().copy_from_slice(image_slice);
            Ok(Binary::from(out_binary))
        })
        .map_err(to_rustler_error)
}

rustler::init!(
    "Elixir.Silicon.Native",
    [nif_format_png, nif_format_rgba8],
    load = load
);
fn load(env: Env, _: Term) -> bool {
    rustler::resource!(ThemeSetResource, env);
    true
}
