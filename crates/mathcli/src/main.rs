//! `mathcli`: render TeX math to PNG or SVG from the command line.
//!
//! Examples:
//!   mathcli 'x^2 + y^2 = r^2' -o out.png
//!   mathcli --svg '\frac{a}{b}' -o out.svg
//!   mathcli --inline --size 20 'e^{i\pi}+1=0' -o e.png

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use mathcore::{Color, RenderOptions};
use mathraster::RasterOptions;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mathcli", about = "Native TeX math renderer")]
struct Args {
    /// TeX math source. Use `-` to read from stdin.
    tex: String,
    /// Output file. Format is inferred from the extension unless --svg is given.
    #[arg(short, long)]
    output: PathBuf,
    /// Emit SVG instead of PNG.
    #[arg(long)]
    svg: bool,
    /// Inline (text style) instead of display style.
    #[arg(long)]
    inline: bool,
    /// Em size in pixels.
    #[arg(long, default_value_t = 48.0)]
    size: f32,
    /// Device pixel ratio for PNG output.
    #[arg(long, default_value_t = 2.0)]
    scale: f32,
    /// Padding around the formula in layout pixels.
    #[arg(long, default_value_t = 8.0)]
    padding: f32,
    /// Transparent background instead of white.
    #[arg(long)]
    transparent: bool,
    /// Print the display list as text for debugging.
    #[arg(long)]
    dump: bool,
    /// Break the formula to fit this width in layout pixels.
    #[arg(long)]
    width: Option<f32>,
    /// Indent of continuation lines, in em.
    #[arg(long, default_value_t = 2.0)]
    indent: f32,
    /// Macro definition `\name=body`, repeatable. `#1`..`#9` are arguments.
    #[arg(long = "macro", value_name = "NAME=BODY")]
    macros: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let tex = if args.tex == "-" {
        let mut s = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut s)?;
        s
    } else {
        args.tex.clone()
    };
    let font = mathcore::bundled::font().map_err(|e| anyhow!("{e}"))?;
    let mut macros = mathcore::Macros::new();
    for m in &args.macros {
        let (name, body) = m.split_once('=').ok_or_else(|| anyhow!("--macro expects NAME=BODY, got `{m}`"))?;
        macros.define(name, body);
    }
    let opts = RenderOptions {
        font_size: args.size,
        display_mode: !args.inline,
        color: Color::BLACK,
        macros,
        line_break: args.width.map(|w| mathcore::LineBreak {
            max_width: w,
            indent: args.indent,
        }),
        budget: mathcore::Budget::default(),
        hit_testing: false,
    };
    let dl = mathcore::render(&font, tex.trim(), &opts).map_err(|e| anyhow!("{e}"))?;
    if args.dump {
        println!("width={} ascent={} descent={}", dl.width, dl.ascent, dl.descent);
        for it in &dl.items {
            println!("{it:?}");
        }
    }
    let is_svg = args.svg || args.output.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"));
    if is_svg {
        std::fs::write(&args.output, mathraster::to_svg(&font, &dl, args.padding)).context("writing svg")?;
    } else {
        let ropts = RasterOptions {
            scale: args.scale,
            padding: args.padding,
            background: if args.transparent { None } else { Some(Color(255, 255, 255, 255)) },
        };
        let png = mathraster::to_png(&font, &dl, &ropts).ok_or_else(|| anyhow!("rasterization failed"))?;
        std::fs::write(&args.output, png).context("writing png")?;
    }
    Ok(())
}
