//! Bounds-checked decoder for Darkstone static O3D geometry.

mod parse;

pub use parse::{O3dLimits, decode_o3d};

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum O3dError {
    #[error("O3D input size {size} exceeds configured limit {limit}")]
    InputLimitExceeded { size: usize, limit: usize },
    #[error("O3D declares {count} vertices, configured limit is {max}")]
    VertexCountLimit { count: u32, max: u32 },
    #[error("O3D declares {count} faces, configured limit is {max}")]
    FaceCountLimit { count: u32, max: u32 },
    #[error("arithmetic overflow while parsing O3D data")]
    ArithmeticOverflow,
    #[error("unexpected end of O3D data at byte offset {offset}")]
    UnexpectedEof { offset: usize },
    #[error("vertex {vertex} axis {axis} is not finite: {value}")]
    NonFinitePosition {
        vertex: usize,
        axis: usize,
        value: f32,
    },
    #[error(
        "face {face} corner {corner} references vertex {index}, but only {vertex_count} vertices exist"
    )]
    VertexIndexOutOfRange {
        face: usize,
        corner: usize,
        index: u16,
        vertex_count: usize,
    },
    #[error("too many neutral vertices to address with u32 indices")]
    NeutralVertexOverflow,
    #[error("too many distinct material slots to address with u16 indices")]
    MaterialSlotOverflow,
    #[error("neutral mesh validation failed: {0}")]
    Mesh(#[from] darkstone_assets::MeshError),
}
