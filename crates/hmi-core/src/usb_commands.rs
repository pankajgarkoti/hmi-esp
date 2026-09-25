//! Diagnostic USB commands must be explicitly framed; other USB clients send text too.
#[derive(Default)]
pub struct UsbCommands {
    line: [u8; 32],
    used: usize,
    overflow: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    BootClick,
    BootHold,
    KeyClick,
    KeyHold,
    Clock,
    Status,
}

impl UsbCommands {
    pub fn feed(&mut self, byte: u8) -> Option<Command> {
        if byte == b'\n' {
            let result = if self.overflow {
                None
            } else {
                match &self.line[..self.used] {
                    b"@LIVING\tBOOT" => Some(Command::BootClick),
                    b"@LIVING\tBOOT_HOLD" => Some(Command::BootHold),
                    b"@LIVING\tKEY" => Some(Command::KeyClick),
                    b"@LIVING\tKEY_HOLD" => Some(Command::KeyHold),
                    b"@LIVING\tCLOCK" => Some(Command::Clock),
                    b"@LIVING\tSTATUS" => Some(Command::Status),
                    _ => None,
                }
            };
            self.used = 0;
            self.overflow = false;
            return result;
        }
        if self.used == self.line.len() {
            self.overflow = true;
        } else if !self.overflow {
            self.line[self.used] = byte;
            self.used += 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrelated_agentdeck_frames_cannot_change_screen() {
        let mut parser = UsbCommands::default();
        let traffic =
            b"@AD2\tHELLO\n@DP2\tS\t123\t-\t50\n@AD1\tC\tconnect\tx\n@DP3\tS\tt\tb\nsbtbBks\n";
        assert!(traffic.iter().all(|b| parser.feed(*b).is_none()));
        assert_eq!(
            b"@LIVING\tCLOCK\n"
                .iter()
                .filter_map(|b| parser.feed(*b))
                .collect::<Vec<_>>(),
            [Command::Clock]
        );
    }

    #[test]
    fn partial_oversized_and_malformed_commands_do_not_execute() {
        let mut parser = UsbCommands::default();
        assert!(b"@LIVING\tBOOT".iter().all(|b| parser.feed(*b).is_none()));
        assert_eq!(parser.feed(b'\n'), Some(Command::BootClick));
        assert!(vec![b'x'; 100]
            .into_iter()
            .chain(b"@LIVING\tBOOT\n".iter().copied())
            .all(|b| parser.feed(b).is_none()));
        assert_eq!(
            b"@LIVING\tKEY\n"
                .iter()
                .filter_map(|b| parser.feed(*b))
                .collect::<Vec<_>>(),
            [Command::KeyClick]
        );
    }
}
