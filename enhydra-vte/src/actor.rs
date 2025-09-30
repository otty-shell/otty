pub trait Actor {
    fn print(&mut self, c: char);

    fn execute(&mut self, byte: u8);

    fn hook(
        &mut self,
        byte: u8,
        params: &[i64],
        intermediates: &[u8],
        ignored_excess_intermediates: bool,
    );

    fn put(&mut self, byte: u8);

    fn unhook(&mut self);

    fn osc_dispatch(&mut self, params: &[&[u8]]);

    fn csi_dispatch(
        &mut self,
        params: &[u8],
        parameters_truncated: bool,
        byte: u8,
    );

    fn esc_dispatch(
        &mut self,
        params: &[i64],
        intermediates: &[u8],
        ignored_excess_intermediates: bool,
        byte: u8,
    );
}
