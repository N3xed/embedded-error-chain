use embedded_error_chain::prelude::*;

#[derive(Clone, Copy, ErrorCategory)]
#[repr(u8)]
enum OtherError {
    ExtremeFailure,
}

static SOME_GLOBAL_VARIABLE: usize = 200;

#[derive(Clone, Copy, ErrorCategory)]
#[error_category(name = "optional name", links(OtherError))]
#[repr(u8)]
enum TestError {
    /// Foo error (summary)
    ///
    /// Detailed description.
    /// The summary and detailed description are available as placeholders in
    /// the `#[error(...)]` attribute. If no such attribute is put on the variant
    /// or the `...` part is empty, then the summary will be used. If the summary
    /// does not exist (no doc comments on the variant), then the variant name is
    /// used for debug printing.
    #[error("format string {summary}, {details}, {variant}, {category}")]
    Foo = 0,

    #[error("custom {}, {:?}", "some_expr", SOME_GLOBAL_VARIABLE)]
    Other,

    /// Some explanation
    Bar,
}

#[derive(Clone, Copy, ErrorCategory)]
#[error_category(links(OtherError, TestError))]
#[repr(u8)]
enum SeperateError {
    SomethingHappened,
}

#[derive(Clone, Copy, ErrorCategory)]
enum YetEmptyError {}

#[test]
fn check_print() {
    assert_eq!(
        format!("{:?}", TestError::Foo),
        "format string Foo error (summary), Detailed description.\nThe summary and detailed description are available as placeholders in\nthe `#[error(...)]` attribute. If no such attribute is put on the variant\nor the `...` part is empty, then the summary will be used. If the summary\ndoes not exist (no doc comments on the variant), then the variant name is\nused for debug printing., Foo, optional name"
    );

    assert_eq!(format!("{:?}", TestError::Other), "custom some_expr, 200");

    let err = (OtherError::ExtremeFailure).chain(TestError::Bar);
    assert_eq!(
        format!("{:?}", err),
        "optional name(2): Some explanation\n- OtherError(0): ExtremeFailure"
    )
}
