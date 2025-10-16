//! otty-surface: screen surface construction from VTE events
//!
//! This crate implements a surface model that consumes events from `otty-vte`
//! (the VTE parser) and builds an in-memory representation of a terminal
//! screen. It exposes a small API to feed bytes and manipulate the surface.
//!
//! Two surface styles are planned:
//! - Grid surface (classic terminal grid like alacritty)
//! - Block surface (output split into blocks like warp) — stubbed for now
//!
//! For now only the grid surface is implemented with a minimal but practical
//! subset of CSI/ESC handling (printables, cursor movement, SGR, clear ops,
//! and basic scrolling).

mod block;
mod cell;
mod color;
mod cursor;
mod grid;

pub use grid::GridSurface;
