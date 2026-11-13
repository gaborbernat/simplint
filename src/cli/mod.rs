pub(crate) mod args;
pub(crate) mod check;
mod config;
pub(crate) mod printer;
pub(crate) mod subcommands;
mod watch;

// Writes each piece and flushes, so a command fails on a full disk or a closed pipe however its output was buffered.
fn write_flushed<P: AsRef<[u8]>>(
    writer: &mut dyn std::io::Write,
    pieces: impl IntoIterator<Item = P>,
) -> std::io::Result<()> {
    pieces.into_iter().try_for_each(|piece| writer.write_all(piece.as_ref())).and_then(|()| writer.flush())
}
