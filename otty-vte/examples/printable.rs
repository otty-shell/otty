use otty_vte::{Actor, CsiParam, Parser};

#[derive(Default)]
struct MyActor;

impl Actor for MyActor {
    fn print(&mut self, c: char) {
        println!("print: {c}");
    }

    fn execute(&mut self, byte: u8) {
        println!("exec: {byte:#04x}");
    }

    fn hook(
        &mut self,
        byte: u8,
        params: &[i64],
        interms: &[u8],
        ignored: bool,
    ) {
        println!(
            "DCS hook: final={byte:#04x} params={params:?} interms={interms:?} ignored={ignored}"
        );
    }

    fn put(&mut self, byte: u8) {
        println!("DCS put: {byte:#04x}");
    }

    fn unhook(&mut self) {
        println!("DCS unhook");
    }

    fn osc_dispatch(&mut self, params: &[&[u8]]) {
        println!("OSC: {:?}", params);
    }

    fn csi_dispatch(
        &mut self,
        params: &[CsiParam],
        _intermediates: &[u8],
        truncated: bool,
        byte: u8,
    ) {
        println!(
            "CSI: params={params:?} truncated={truncated} final={byte:#04x}"
        );
    }

    fn esc_dispatch(
        &mut self,
        params: &[i64],
        interms: &[u8],
        ignored: bool,
        byte: u8,
    ) {
        println!(
            "ESC: params={params:?} interms={interms:?} ignored={ignored} final={byte:#04x}"
        );
    }
}

fn main() {
    let mut parser = Parser::new();
    let mut actor = MyActor::default();
    parser.advance(b"\x1b[31mhi\x1b[0m", &mut actor);
}
