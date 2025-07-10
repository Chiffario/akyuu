bitflags::bitflags! {
    pub struct CommandFlags: u8 {
        const EPHEMERAL   = 1 << 0;
        const SKIP_DEFER  = 1 << 1;
    }
}

impl CommandFlags {
    pub fn defer(self) -> bool {
        !self.contains(CommandFlags::SKIP_DEFER)
    }

    pub fn ephemeral(self) -> bool {
        self.contains(CommandFlags::EPHEMERAL)
    }
}
