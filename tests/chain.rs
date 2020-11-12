use embedded_error_chain::{marker::Unused, *};
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
enum TestError1 {
    Err0 = 0,
    Err1 = 1,
}

impl ErrorCategory for TestError1 {
    const NAME: &'static str = "ErrorCategory";

    type L0 = Unused;
    type L1 = Unused;
    type L2 = Unused;
    type L3 = TestError3;
    type L4 = Unused;
    type L5 = Unused;
}

impl From<ErrorCode> for TestError1 {
    fn from(val: ErrorCode) -> Self {
        assert!(val <= TestError1::Err1 as u8);
        unsafe { core::mem::transmute(val) }
    }
}

impl Into<ErrorCode> for TestError1 {
    fn into(self) -> ErrorCode {
        self as ErrorCode
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
enum TestError2 {
    Err0 = 0,
    Err1 = 1,
}

impl ErrorCategory for TestError2 {
    const NAME: &'static str = "ErrorCategory";

    type L0 = TestError1;
    type L1 = Unused;
    type L2 = Unused;
    type L3 = Unused;
    type L4 = Unused;
    type L5 = Unused;
}

impl From<ErrorCode> for TestError2 {
    fn from(val: ErrorCode) -> Self {
        assert!(val <= TestError2::Err1 as u8);
        unsafe { core::mem::transmute(val) }
    }
}

impl Into<ErrorCode> for TestError2 {
    fn into(self) -> ErrorCode {
        self as ErrorCode
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
enum TestError3 {
    Err0 = 3,
    Err1 = 4,
}

impl ErrorCategory for TestError3 {
    const NAME: &'static str = "ErrorCategory";

    type L0 = TestError3;
    type L1 = TestError1;
    type L2 = TestError2;
    type L3 = Unused;
    type L4 = Unused;
    type L5 = Unused;
}

impl From<ErrorCode> for TestError3 {
    fn from(val: ErrorCode) -> Self {
        assert!(val <= TestError3::Err1 as u8);
        assert!(val >= TestError3::Err0 as u8);
        unsafe { core::mem::transmute(val) }
    }
}

impl Into<ErrorCode> for TestError3 {
    fn into(self) -> ErrorCode {
        self as ErrorCode
    }
}

#[test]
fn chain1() {
    let err = Error::new(TestError1::Err0);
    let err = err.chain(TestError2::Err1);

    assert_eq!(err.code(), TestError2::Err1);
    assert_eq!(
        err.code_of_category::<TestError1>().unwrap(),
        TestError1::Err0
    );
    assert_eq!(err.chain_len(), 1);

    let mut iter = err.iter();
    assert_eq!(
        iter.next(),
        Some((
            TestError2::Err1.into(),
            ErrorCategoryHandle::new::<TestError2>()
        ))
    );
    assert_eq!(
        iter.next(),
        Some((
            TestError1::Err0.into(),
            ErrorCategoryHandle::new::<TestError1>()
        ))
    );
    assert_eq!(iter.next(), None);
}

#[test]
fn chain2() {
    let err = Error::new(TestError2::Err1)
        .chain(TestError3::Err0)
        .chain(TestError3::Err1)
        .chain(TestError1::Err1)
        .chain(TestError2::Err0);

    let mut iter = err.iter();
    assert_eq!(
        iter.next(),
        Some((
            TestError2::Err0.into(),
            ErrorCategoryHandle::new::<TestError2>()
        ))
    );
    assert_eq!(
        iter.next(),
        Some((
            TestError1::Err1.into(),
            ErrorCategoryHandle::new::<TestError1>()
        ))
    );
    assert_eq!(
        iter.next(),
        Some((
            TestError3::Err1.into(),
            ErrorCategoryHandle::new::<TestError3>()
        ))
    );
    assert_eq!(
        iter.next(),
        Some((
            TestError3::Err0.into(),
            ErrorCategoryHandle::new::<TestError3>()
        ))
    );
    assert_eq!(
        iter.next(),
        Some((
            TestError2::Err1.into(),
            ErrorCategoryHandle::new::<TestError2>()
        ))
    );
    assert_eq!(iter.next(), None);
}

#[test]
#[should_panic(expected = "chaining two errors overflowed; error chain is full")]
fn chain3() {
    let _err = Error::new(TestError2::Err1)
        .chain(TestError3::Err0)
        .chain(TestError3::Err1)
        .chain(TestError1::Err1)
        .chain(TestError2::Err1)
        .chain(TestError3::Err0);
}
