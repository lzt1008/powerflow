#[macro_export]
macro_rules! cfstr {
    ($literal:literal) => {
        core_foundation::string::CFString::from_static_string($literal)
    };

    ($expr:expr) => {
        core_foundation::string::CFString::new($expr)
    };
}

#[macro_export]
macro_rules! cfdic {
    ($($key:literal = $value:literal)*) => {
        core_foundation::dictionary::CFDictionary::from_CFType_pairs(&[
            $(($crate::cfstr!($key), $crate::cfstr!($value))),*
        ])
    };
}
