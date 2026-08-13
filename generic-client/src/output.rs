use crate::Connectivity;

#[derive(Debug)]
#[must_use]
pub enum Output {
    ConnectivityChanged {
        connectivity: Connectivity,
    },
    NewText {
        text: String,
    },
    Both {
        connectivity: Connectivity,
        text: String,
    },
}
