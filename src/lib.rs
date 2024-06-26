pub mod expr;
pub mod builtin;
pub mod parse;
pub mod typing;
pub mod ir1;
pub mod ir1_egglog;
pub mod ir2;
pub mod util;
pub mod wasm;
pub mod runtime;
pub mod toplevel;

#[cfg(target_arch = "wasm32")]
pub mod bindings;
