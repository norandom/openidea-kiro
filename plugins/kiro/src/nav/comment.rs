//! Comment syntax determination and line wrapping for code files (task 2.1).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommentStyle {
    DoubleSlash,
    Hash,
    DoubleDash,
    Html,
    Bare,
}

pub fn style_for(_path: &str) -> CommentStyle {
    CommentStyle::Bare
}

pub fn wrap(_style: CommentStyle, text: &str) -> String {
    text.to_string()
}
