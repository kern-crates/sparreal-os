use core::ptr::NonNull;

use alloc::{format, string::String, vec::Vec};
use driver_interface::{DriverRegister, ProbeFnKind, timer::*};
use fdt_parser::Fdt;

use crate::prelude::GetIrqConfig;

use super::{Descriptor, Device, DeviceId};

pub struct Container {
    data: Option<Device<Driver>>,
}

impl Container {
    pub const fn new() -> Self {
        Self { data: None }
    }

    pub fn set(&mut self, device: Device<Driver>) {
        self.data = Some(device);
    }

    pub fn get_cpu_timer(&self) -> Option<Device<PerCPU>> {
        if let Some(device) = self.data.as_ref() {
            loop {
                if let Ok(mut d) = device.try_use_by("cpu") {
                    let p = d.get_current_cpu();
                    let mut desc = d.descriptor.clone();
                    desc.device_id = Default::default();
                    return Some(Device::new(desc, p));
                }
            }
        }

        None
    }
}

pub fn init_by_fdt(
    registers: &[DriverRegister],
    fdt_addr: NonNull<u8>,
) -> Result<Device<Driver>, String> {
    let fdt = Fdt::from_ptr(fdt_addr).map_err(|e| format!("{e:?}"))?;
    for r in registers {
        if let ProbeFnKind::Timer(probe) = r.probe {
            let compa = r
                .compatibles
                .split("\n")
                .filter_map(|e| if e.is_empty() { None } else { Some(e) })
                .collect::<Vec<_>>();
            for node in fdt.find_compatible(&compa) {
                let irq = match node.irq_info() {
                    Some(irq) => irq,
                    None => continue,
                };

                let timer = probe(irq.cfgs.clone());
                let dev = Device::new(
                    Descriptor {
                        name: timer.name(),
                        irq: Some(irq),
                        ..Default::default()
                    },
                    timer,
                );

                return Ok(dev);
            }
        }
    }
    Err("No timer found".into())
}
