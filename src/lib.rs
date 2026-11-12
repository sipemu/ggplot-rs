pub mod aes;
pub mod annotate;
pub mod build;
pub mod compose;
pub mod coord;
pub mod data;
pub mod facet;
pub mod format;
pub mod geom;
pub mod ggpubr;
pub mod graph;
pub mod graph_layout;
pub mod guide;
pub mod helpers;
pub mod locale;
pub mod plot;
pub mod position;
pub mod prelude;
pub mod render;
pub(crate) mod rng;
pub mod scale;
#[cfg(feature = "sf")]
pub mod spatial;
#[cfg(feature = "spec")]
pub mod spec;
pub mod stat;
pub mod summary;
pub mod theme;

pub use locale::Locale;

/// Compiles the code blocks of the agent skill shipped in `skills/ggplot-rs/`
/// as doctests, so the skill's examples can't drift from the API.
#[cfg(doctest)]
#[doc = include_str!("../skills/ggplot-rs/SKILL.md")]
pub struct RustSkillExamples;
pub use plot::{GGError, GGPlot};
