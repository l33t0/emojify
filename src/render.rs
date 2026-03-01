//! Rendering pipeline for emoji generation.

mod composite;
mod gif;
mod gradient;
mod image;
mod split;
mod text;

pub use composite::{Anchor, OverlaySpec, composite};
pub use gif::{GifOptions, encode_animated_gif, generate_pulse_animation};
pub use gradient::{GradientSpec, apply_gradient_to_text, generate_gradient};
pub use image::{encode_output, load_and_resize_image, load_image_from_bytes, resize_image_to_fit};
pub use split::{format_emoji_grid, split_image};
pub use text::{TextRenderOptions, render_text};
