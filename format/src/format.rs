
/// 检测 TAS 脚本内容是否含有断点(`***`)以及其所在行位置
pub(crate) fn find_breakpoints(content: &str) -> impl Iterator<Item = (usize, &str)> {
    filter_lines(content, |(_, s)| s.contains("***"))
}

pub(crate) fn find_start_labels(content: &str) -> impl Iterator<Item = (usize, &str)> {
    filter_lines(content, |(_, s)| s.starts_with("#Start"))
}

fn filter_lines(
    content: &str,
    predicate: impl FnMut(&(usize, &str)) -> bool,
) -> impl Iterator<Item = (usize, &str)> {
    content
        .lines()
        .enumerate()
        .filter(predicate)
        .map(|(ln, a)| (ln + 1, a))
}
