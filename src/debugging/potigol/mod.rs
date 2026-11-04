mod classic;
mod jerimum;

pub(super) fn prefix(source: &str, line: usize) -> String {
    if source.lines().any(|line| line.trim() == "use \"jerimum\"") {
        jerimum::prefix(source, line)
    } else {
        classic::prefix(source, line)
    }
}
