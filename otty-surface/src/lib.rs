//! Terminal surface abstraction for grid‑based terminal content.

mod actor;
mod cell;
mod color;
mod damage;
mod grid;
mod hyperlink;
mod index;
mod mode;
mod search;
mod selection;
mod snapshot;
mod surface;

pub(crate) use otty_escape as escape;

pub use actor::SurfaceActor;
pub use cell::{Cell, Flags};
pub use color::Colors;
pub use grid::{Dimensions, Grid, Scroll};
pub use index::{Column, Line, Point, Side};
pub use mode::SurfaceMode;
pub use search::{Match, RegexIter, RegexSearch};
pub use selection::{SelectionRange, SelectionType};
pub use snapshot::{
    CursorSnapshot, SnapshotCell, SnapshotDamage, SnapshotOwned, SnapshotSize,
    SnapshotView, SurfaceModel,
};
pub use surface::{
    Surface, SurfaceConfig, point_to_viewport, viewport_to_point,
};
