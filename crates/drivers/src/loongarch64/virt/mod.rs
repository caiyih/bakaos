use abstractions::IUsizeAlias;
use address::{IConvertablePhysicalAddress, PhysicalAddress};
use alloc::sync::Arc;
use timing::TimeSpec;

use crate::{BlockDeviceInode, IMachine};

#[derive(Clone, Copy)]
pub struct VirtMachine;

impl IMachine for VirtMachine {
    fn name(&self) -> &'static str {
        "QEMU Virt Machine(LoongArch64)"
    }

    fn query_performance_frequency(&self) -> u64 {
        // We can read with the instruction below
        // cpucfg rd, 0x4   // CC_FREQ
        100_000_000
    }

    fn mmio(&self) -> &[(usize, usize)] {
        &[
            (0x100E_0000, 0x0000_1000), // GED
            (0x1FE0_0000, 0x0000_1000), // UART
            (0x2000_0000, 0x1000_0000), // PCI
            (0x4000_0000, 0x0002_0000), // PCI RANGES
        ]
    }

    fn memory_end(&self) -> usize {
        // 128M
        0x8800_0000
    }

    fn query_performance_counter(&self) -> usize {
        platform_specific::stable_counter()
    }

    fn get_rtc_offset(&self) -> timing::TimeSpec {
        // loongson,ls7a-rtc
        // https://github.com/qemu/qemu/blob/661c2e1ab29cd9c4d268ae3f44712e8d421c0e56/include/hw/pci-host/ls7a.h#L45
        const RTC_BASE: usize = 0x100D0100;
        const RTC_CNT: usize = 0x68 / 4;
        const TOY_LOW: usize = 0x2C / 4;
        const TOY_HIGH: usize = 0x30 / 4; // year - 1900

        // https://github.com/qemu/qemu/blob/661c2e1ab29cd9c4d268ae3f44712e8d421c0e56/hw/rtc/ls7a_rtc.c#L37
        const RTC_FREQ: u64 = 32768;

        let rtc_base = PhysicalAddress::from_usize(RTC_BASE);
        let rtc_base = rtc_base.to_high_virtual() & 0x8000_FFFF_FFFF_FFFF;
        let rtc_base = unsafe { rtc_base.as_ptr::<u32>() };

        let rtc_cnt = unsafe { rtc_base.add(RTC_CNT).read_volatile() };
        let pmcnt = self.query_performance_counter();

        let toy_low = unsafe { rtc_base.add(TOY_LOW).read_volatile() };
        let rtc_cnt_again = unsafe { rtc_base.add(RTC_CNT).read_volatile() };

        let year_reg = unsafe { rtc_base.add(TOY_HIGH).read_volatile() };

        let year = year_reg + 1900;

        fn extract_low_field(val: u32, shift: usize, len: usize) -> i64 {
            ((val as usize >> shift) & ((1 << len) - 1)) as i64
        }

        // see https://github.com/qemu/qemu/blob/661c2e1ab29cd9c4d268ae3f44712e8d421c0e56/hw/rtc/ls7a_rtc.c#L43-L48
        let month = extract_low_field(toy_low, 26, 6);
        let day = extract_low_field(toy_low, 21, 5);
        let hour = extract_low_field(toy_low, 16, 5);
        let min = extract_low_field(toy_low, 10, 6);
        let mut sec = extract_low_field(toy_low, 4, 6);
        // let msec = extract_low_field(toy_low, 0, 4); // msec is always unavaliable, so we use rtc time instead.

        // cross seconds handling due to overflowing, we assume this happens only once at most
        if rtc_cnt_again < rtc_cnt {
            sec += 1;
        }

        let rtc_msec = rtc_cnt % RTC_FREQ as u32;
        let rtc_time = TimeSpec::from_ticks(rtc_msec as i64, RTC_FREQ);
        let toy_time = toy_to_timestamp(year as i64, month, day, hour, min, sec);

        let pmcnt_time = TimeSpec::from_ticks(pmcnt as i64, self.query_performance_frequency());

        rtc_time + toy_time - pmcnt_time
    }

    fn create_block_device_at(&self, _device_id: usize) -> Arc<BlockDeviceInode> {
        todo!()
    }
}

fn toy_to_timestamp(year: i64, month: i64, day: i64, hour: i64, min: i64, sec: i64) -> TimeSpec {
    fn days_since_1970(year: i64, month: i64, day: i64) -> i64 {
        fn is_leap_year(year: i64) -> bool {
            (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
        }

        fn days_in_month(month: i64, year: i64) -> i64 {
            const DAYS: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
            if month == 2 && is_leap_year(year) {
                29
            } else {
                DAYS[(month - 1) as usize]
            }
        }

        let mut days = day - 1;

        for y in 1970..year {
            days += if is_leap_year(y) { 366 } else { 365 };
        }

        for m in 1..month {
            days += days_in_month(m, year);
        }

        days
    }

    let days = days_since_1970(year, month, day);
    let tv_sec = (days * 24 + hour) * 3600 + min * 60 + sec;

    TimeSpec { tv_sec, tv_nsec: 0 }
}
