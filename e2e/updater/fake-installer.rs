// Stand-in for the Codestamp installer in the update test: records how it was started, then exits.
use std::io::Write;

fn main() {
    let Ok(log) = std::env::var("CODESTAMP_FAKE_INSTALLER_LOG") else { return };
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(log) {
        let _ = writeln!(file, "{}", args.join(" "));
    }
}
