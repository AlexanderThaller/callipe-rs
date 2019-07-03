use crate::input::Input;
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub(super) struct Settings {
    pub(super) inputs: Vec<Input>,
}
