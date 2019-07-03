use crate::input::InputCollector;
use std::{
    thread,
    time,
};
use systemstat::{
    Platform,
    System,
};

pub(super) struct Cpu {
    pub(super) per_cpu: bool,
    pub(super) total_cpu: bool,
}

impl InputCollector for Cpu {
    fn run(self) -> Result<(), String> {
        thread::spawn(|| loop {
            let sys = System::new();

            match sys.cpu_load() {
                Ok(cpus) => {
                    println!("\nMeasuring CPU load...");
                    thread::sleep(time::Duration::from_secs(1));

                    let cpus = cpus.done().unwrap();

                    for (index, cpu) in cpus.iter().enumerate() {
                        println!(
                            "CPU load for cpu{}: {}% user, {}% nice, {}% system, {}% intr, {}% \
                             idle ",
                            index,
                            cpu.user * 100.0,
                            cpu.nice * 100.0,
                            cpu.system * 100.0,
                            cpu.interrupt * 100.0,
                            cpu.idle * 100.0
                        );
                    }
                }
                Err(x) => println!("\nCPU load: error: {}", x),
            };
        });

        Ok(())
    }
}
