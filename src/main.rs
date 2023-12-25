use std::io::{self, stdout, Read};
use termion::raw::IntoRawMode;

fn die(e: std::io::Error) {
    panic!("{}", e);
}

fn main() {
    let _stdout = stdout().into_raw_mode().unwrap();
    for b in io::stdin().bytes() {
        match b {
            Ok(b) => { // similar to `let b = b.unwrap()` - shadows original b, not same obj as iterator
                let c = b as char;
                if c.is_control() {
                    println!("{:?} \r", b);
                } else {
                    println!("{:?} ({})\r", b, c);
                }

                if b == ('q' as u8 - 'a' as u8 + 1) { // CTRL+Q == 17 (one-indexed), q - a = 16
                    break;
                }
            }
            Err(err) => die(err),
        }
    }
}
