//! Read-only boot disk and a single-byte interrupt-driven UART.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::Machine;
use crate::core::error::{self, Result};

const DISK_LBA: u8 = 0x20;
const DISK_DATA: u8 = 0x21;
const DISK_EOF: i64 = 0x100;
pub const BOOT_ADDRESS: usize = 0x2000;
pub const SECTOR_SIZE: usize = 512;

pub fn attach_boot_disk(machine: &mut Machine, sector: [u8; SECTOR_SIZE]) {
    let cursor = Rc::new(Cell::new(None));
    let select = Rc::clone(&cursor);
    machine.attach_output(DISK_LBA, move |sector| select.set((sector == 0).then_some(0)));
    machine.attach_input(DISK_DATA, move || {
        let Some(index) = cursor.get() else {
            return DISK_EOF;
        };
        cursor.set((index + 1 < SECTOR_SIZE).then_some(index + 1));
        i64::from(sector[index])
    });
}

#[derive(Default)]
pub struct Uart {
    pending: Option<u8>,
    polls_until_ready: u8,
    received: Vec<u8>,
    overrun: bool,
}

impl Uart {
    pub fn attach(machine: &mut Machine, port: u8) -> Rc<RefCell<Self>> {
        let uart = Rc::new(RefCell::new(Self::default()));
        let transmitter = Rc::clone(&uart);
        machine.attach_output(port, move |value| {
            let mut uart = transmitter.borrow_mut();
            if uart.pending.is_some() {
                uart.overrun = true;
            } else {
                uart.pending = Some(value as u8);
                uart.polls_until_ready = 1;
            }
        });
        let interrupt = Rc::clone(&uart);
        machine.attach_interrupt(move || {
            let mut uart = interrupt.borrow_mut();
            // Give the virtual transmitter one idle poll before raising its interrupt.
            if uart.polls_until_ready != 0 {
                uart.polls_until_ready -= 1;
                return false;
            }
            let Some(byte) = uart.pending.take() else {
                return false;
            };
            uart.received.push(byte);
            true
        });
        uart
    }

    pub fn frame(&self) -> Result<&[u8]> {
        if self.overrun || self.pending.is_some() {
            return Err(error::machine_fault("UART transmission did not complete cleanly"));
        }
        Ok(&self.received)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::runtime::assemble;

    const ROM: &str = include_str!("firmware/init.asm");
    const GUEST: &str = include_str!("firmware/greeting.asm");

    #[test]
    fn rom_rejects_a_disk_without_the_boot_signature() {
        let mut machine = Machine::new();
        let status = Rc::new(Cell::new(None));
        let report = Rc::clone(&status);
        machine.attach_output(2, move |value| report.set(Some(value)));
        attach_boot_disk(&mut machine, [0; SECTOR_SIZE]);
        machine.load(&assemble(ROM).unwrap().image, 0).unwrap();

        machine.run(25_000).unwrap();
        assert_eq!(status.get(), Some(0xBAD));
        assert!(usize::from(machine.pc) < BOOT_ADDRESS);
    }

    #[test]
    fn guest_sends_the_complete_frame_through_uart_interrupts() {
        let mut machine = Machine::new();
        let guest = assemble(GUEST).unwrap();
        let uart = Uart::attach(&mut machine, 1);
        machine.attach_output(2, |status| assert_eq!(status, 0x600D));
        attach_boot_disk(&mut machine, guest.image[BOOT_ADDRESS..].try_into().unwrap());
        machine.load(&assemble(ROM).unwrap().image, 0).unwrap();
        let digits: Vec<u8> = b"31415926535".iter().map(|digit| digit - b'0').collect();
        machine.load(&digits, 0x8000).unwrap();

        machine.run(25_000).unwrap();
        assert_eq!(uart.borrow().frame().unwrap(), b"hello world\n");
        assert_eq!(machine.registers[7], 12);
        assert_eq!(machine.sp, 0xFFFE);
    }
}
