//! Terminal surface abstraction for grid‑based terminal content.

mod actor;
mod cell;
mod color;
mod damage;
mod grid;
mod index;
mod mode;
mod selection;
mod snapshot;
mod surface;

pub(crate) use otty_escape as escape;

pub use actor::SurfaceActor;
pub use cell::{Cell, Flags};
pub use grid::{Dimensions, Grid, Scroll};
pub use index::{Column, Line, Point, Side};
pub use mode::SurfaceMode;
pub use selection::SelectionType;
pub use snapshot::{CursorSnapshot, SurfaceSnapshot, SurfaceSnapshotSource};
pub use surface::{
    Surface, SurfaceConfig, point_to_viewport, viewport_to_point,
};
