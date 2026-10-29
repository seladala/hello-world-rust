pub mod compiler;
pub mod executor;
pub mod inspector;
pub mod opcodes;

pub use compiler::{assemble, AssembledProgram};
pub use executor::Machine;
pub use inspector::disassemble;
