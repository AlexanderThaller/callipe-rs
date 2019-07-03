mod cpu;

use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Serialize, Deserialize)]
pub(super) enum Input {
    Cpu { per_cpu: bool, total_cpu: bool },
}

trait InputCollector {
    fn run(self) -> Result<(), String>;
}

impl Input {
    pub(super) fn run(self) -> Result<(), String> {
        match self {
            Input::Cpu { per_cpu, total_cpu } => cpu::Cpu { per_cpu, total_cpu }.run(),
        }
    }
}
