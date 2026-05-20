mod string_utils;

mod bit_array;
pub use bit_array::*;

mod bit_matrix;
pub use bit_matrix::*;

mod bit_source;
pub use bit_source::*;

mod perspective_transform;
pub use perspective_transform::*;

mod grid_sampler;
pub use grid_sampler::*;

mod default_grid_sampler;
pub use default_grid_sampler::*;

mod character_set;
pub use character_set::*;

mod eci_string_builder;
pub use eci_string_builder::*;

mod global_histogram_binarizer;
pub use global_histogram_binarizer::*;

mod hybrid_binarizer;
pub use hybrid_binarizer::*;

mod eci;
pub use eci::*;

mod quad;
pub use quad::*;

pub mod detect;

pub type BitFieldBaseType = usize;
pub const BIT_FIELD_BASE_BITS: usize = BitFieldBaseType::BITS as usize;
pub const BIT_FIELD_SHIFT_BITS: usize = BIT_FIELD_BASE_BITS - 1;
