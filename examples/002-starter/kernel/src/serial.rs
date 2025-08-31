use spin::Mutex;
use x86_64::instructions::port::Port;

pub struct SerialPort {
    data: Port<u8>,
    int_enable: Port<u8>,
    fifo_ctrl: Port<u8>,
    line_ctrl: Port<u8>,
    modem_ctrl: Port<u8>,
    line_status: Port<u8>,
}

impl SerialPort {
    const COM1: u16 = 0x3F8;

    pub const fn new() -> Self {
        unsafe {
            SerialPort {
                data: Port::new(Self::COM1),
                int_enable: Port::new(Self::COM1 + 1),
                fifo_ctrl: Port::new(Self::COM1 + 2),
                line_ctrl: Port::new(Self::COM1 + 3),
                modem_ctrl: Port::new(Self::COM1 + 4),
                line_status: Port::new(Self::COM1 + 5),
            }
        }
    }

    pub fn init(&mut self) {
        unsafe {
            self.int_enable.write(0x00);       // disable interrupts
            self.line_ctrl.write(0x80);        // enable DLAB
            // Baud divisor for 115200 (divisor=1)
            self.data.write(0x01);
            self.int_enable.write(0x00);
            self.line_ctrl.write(0x03);        // 8 bits, no parity, one stop bit
            self.fifo_ctrl.write(0xC7);        // enable FIFO, clear, 14-byte threshold
            self.modem_ctrl.write(0x0B);       // IRQs enabled, RTS/DSR set
        }
    }

    fn can_send(&mut self) -> bool {
        unsafe {
            // Bit 5 = THR empty
            (self.line_status.read() & 0x20) != 0
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        while !self.can_send() {}
        unsafe { self.data.write(byte); }
    }

    pub fn write_str(&mut self, s: &str) {
        for b in s.bytes() {
            if b == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(b);
        }
    }
}

static SERIAL1: Mutex<SerialPort> = Mutex::new(SerialPort::new());

pub fn init() {
    SERIAL1.lock().init();
}

pub fn println(s: &str) {
    SERIAL1.lock().write_str(s);
    SERIAL1.lock().write_str("\n");
}
