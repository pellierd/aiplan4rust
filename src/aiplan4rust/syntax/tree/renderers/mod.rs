pub mod kind;
pub mod default;
pub mod tree;

pub mod syntax;

pub use kind::Kind as RenderKind;

pub use default::render as default_rendering;
pub use tree::render as tree_rendering;
pub use syntax::syntax::render as syntax_rendering;
