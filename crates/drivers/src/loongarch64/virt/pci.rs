use virtio_drivers::transport::{
    pci::{
        bus::{
            BarInfo, Cam, Command, ConfigurationAccess, DeviceFunction, HeaderType, MemoryBarType,
            MmioCam, PciRoot,
        },
        virtio_device_type, PciTransport,
    },
    DeviceType, Transport,
};

pub struct PciRangeAllocator {
    _start: usize,
    end: usize,
    current: usize,
}

impl PciRangeAllocator {
    /// Creates a new allocator from a memory range.
    pub const fn new(base: usize, size: usize) -> Self {
        Self {
            _start: base,
            end: base + size,
            current: base,
        }
    }
}

impl PciRangeAllocator {
    /// Allocates a memory region with the given size.
    ///
    /// The `size` should be a power of 2, and the returned value is also a
    /// multiple of `size`.
    pub fn alloc(&mut self, size: usize) -> Option<usize> {
        if !size.is_power_of_two() {
            return None;
        }
        let ret = align_up(self.current, size);
        if ret + size > self.end {
            return None;
        }

        self.current = ret + size;
        Some(ret)
    }
}

const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

pub fn enumerate_pci_search(
    mmconfig_base: *mut u8,
    pci_base: usize,
    pci_size: usize,
) -> Option<PciTransport> {
    const PCI_BUS_END: usize = 0;

    log::info!("mmconfig_base = {:#x}", mmconfig_base as usize);

    let mut pci_allocator = PciRangeAllocator::new(pci_base, pci_size);
    let mut root = PciRoot::new(unsafe { MmioCam::new(mmconfig_base, Cam::Ecam) });

    for bus in 0..=PCI_BUS_END as u8 {
        for (bdf, dev_info) in root.enumerate_bus(bus) {
            log::debug!("PCI {bdf}: {dev_info}");

            if dev_info.header_type != HeaderType::Standard {
                continue;
            }

            if virtio_device_type(&dev_info).is_some() {
                configure_pci_device(&mut root, bdf, &mut pci_allocator);

                if let Ok(mut transport) =
                    PciTransport::new::<super::hal::VirtHal, _>(&mut root, bdf)
                {
                    log::info!(
                        "Detected virtio PCI device with device type {:?}, features {:#018x}",
                        transport.device_type(),
                        transport.read_device_features(),
                    );

                    if transport.device_type() == DeviceType::Block {
                        return Some(transport);
                    }
                }
            }
        }
    }

    None
}

fn configure_pci_device(
    root: &mut PciRoot<impl ConfigurationAccess>,
    device_function: DeviceFunction,
    allocator: &mut PciRangeAllocator,
) {
    const PCI_BAR_NUM: u8 = 6;

    let mut bar = 0;
    while bar < PCI_BAR_NUM {
        let info = root.bar_info(device_function, bar).unwrap();
        if let BarInfo::Memory {
            address_type,
            address,
            size,
            ..
        } = info
        {
            // if the BAR address is not assigned, call the allocator and assign it.
            if size > 0 && address == 0 {
                let new_addr = allocator
                    .alloc(size as _)
                    .expect("No memory ranges available for PCI BARs!");
                if address_type == MemoryBarType::Width32 {
                    root.set_bar_32(device_function, bar, new_addr as u32);
                } else if address_type == MemoryBarType::Width64 {
                    root.set_bar_64(device_function, bar, new_addr as u64);
                }
            }
        }

        // read the BAR info again after assignment.
        let info = root.bar_info(device_function, bar).unwrap();
        match info {
            BarInfo::IO { address, size } => {
                if address > 0 && size > 0 {
                    log::debug!("  BAR {}: IO  [{:#x}, {:#x})", bar, address, address + size);
                }
            }
            BarInfo::Memory {
                address_type,
                prefetchable,
                address,
                size,
            } => {
                if address > 0 && size > 0 {
                    log::debug!(
                        "  BAR {}: MEM [{:#x}, {:#x}){}{}",
                        bar,
                        address,
                        address + size as u64,
                        if address_type == MemoryBarType::Width64 {
                            " 64bit"
                        } else {
                            ""
                        },
                        if prefetchable { " pref" } else { "" },
                    );
                }
            }
        }

        bar += 1;
        if info.takes_two_entries() {
            bar += 1;
        }
    }

    // Enable the device.
    let (_status, cmd) = root.get_status_command(device_function);
    root.set_command(
        device_function,
        cmd | Command::IO_SPACE | Command::MEMORY_SPACE | Command::BUS_MASTER,
    );
}
