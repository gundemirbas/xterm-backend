use core::panic::PanicInfo;

pub fn init() {}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
