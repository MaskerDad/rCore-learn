//! Implementation of timer-related

use crate::config::CLOCK_FREQ; // The increment of the counter per second
use crate::sbi::set_timer;
use riscv::register::time;

const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1000;

/// read the value of `mtime`
pub fn get_time() -> usize {
    time::read()
}

/// get current time in milliseconds
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

/// set the next timer interrupt
pub fn set_next_trigger() {
    // set 10ms
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC);
}
