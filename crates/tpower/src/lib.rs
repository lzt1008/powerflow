pub mod de;
pub mod ffi;
pub mod macros;
pub mod provider;
pub mod util;

#[cfg(test)]
mod ioreg_smoke {
    #[test]
    fn read_local_ioreg() {
        let r = crate::provider::get_mac_ioreg();
        eprintln!("{r:?}");
        assert!(r.is_ok(), "get_mac_ioreg failed: {r:?}");
    }
}
