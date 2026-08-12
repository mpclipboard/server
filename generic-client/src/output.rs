use crate::Connectivity;

/// Result of reading
#[derive(Debug)]
#[must_use]
pub enum Output {
    /// An event indicating that connectivity changed, guaranteed to be different from a previous one
    ConnectivityChanged {
        /// New connecivity
        connectivity: Connectivity,
    },
    /// New text clip
    NewText {
        /// New text
        text: String,
    },
    /// Connectivity changed and a new text clip was received by the same read
    Both {
        /// New connecivity
        connectivity: Connectivity,
        /// New text
        text: String,
    },
}
