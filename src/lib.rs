mod aes256;
// mod api;
mod cbd;
mod fips202;
mod indcpa;
mod params;
mod poly;
mod polyvec;
mod ntt;
mod reduce;


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
