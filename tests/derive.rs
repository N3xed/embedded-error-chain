#![allow(dead_code)]
use embedded_error_chain::*;

#[derive(Clone, Copy, ErrorCategory)]
#[error_category(links(SeperateError))]
enum OtherError {
    Extreme = 4,
}

const SOME_GLOBAL_VARIABLE: usize = 5;

#[derive(Clone, Copy, ErrorCategory)]
#[error_category(name = "optional name", links(OtherError))]
#[repr(u8)]
enum TestError {
    /// This is a summary
    /// multiline
    #[error("format string {summary}, {details}, {variant}, {category}")]
    Foo,

    #[error("custom {}, {:?}", "some_expr", SOME_GLOBAL_VARIABLE)]
    Other,

    /// Summary
    ///
    /// Detailed description.
    /// The summary and detailed description are available as placeholders in
    /// the `#[error(...)]` attribute. If no such attribute is put on the variant
    /// or the `...` part is empty, then the summary will be used. If the summary
    /// does not exist (no doc comments on the variant), then the variant name is
    /// used for debug printing.
    Bar,
}

#[derive(Clone, Copy, ErrorCategory)]
#[error_category(links(OtherError, TestError))]
enum SeperateError {
    /// Very bad
    ///
    /// This is a very long and details explanation of what happend
    /// and why, all the newlines are preseved and we opt-in so that
    /// this message will be displayed.
    #[error("{details}")]
    SomethingHappened,
}

#[derive(Clone, Copy, ErrorCategory)]
enum YetEmptyError {}

#[test]
fn test() {
    let ec = Error::new(SeperateError::SomethingHappened)
        .chain(OtherError::Extreme)
        .chain(TestError::Bar);

    println!("{:?}", ec);

    let mut iter = ec.iter();
    assert_eq!(
        iter.next(),
        Some((
            TestError::Bar.into(),
            ErrorCategoryHandle::new::<TestError>()
        ))
    );
    assert_eq!(
        iter.next(),
        Some((
            OtherError::Extreme.into(),
            ErrorCategoryHandle::new::<OtherError>()
        ))
    );
    assert_eq!(
        iter.next(),
        Some((
            SeperateError::SomethingHappened.into(),
            ErrorCategoryHandle::new::<SeperateError>()
        ))
    );
    assert_eq!(iter.next(), None);
}
