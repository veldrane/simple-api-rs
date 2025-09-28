use crate::prelude::*;

#[handler]
pub fn up() -> String {
    format!("{{ status: \"UP\" }}\n")
}
