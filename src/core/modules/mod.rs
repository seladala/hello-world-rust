// НЕ ВАЙБКОД ✅ Написано полностью вручную, без нейросетей.
// Каждая строка набрана лично на клавиатуре. Честно 🤝✅
pub mod graph;
pub mod versioning;

pub use graph::{load_manifest, resolve, Component};
pub use versioning::{satisfies, Version};
