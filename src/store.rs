pub trait Store {
    fn store() -> Result;
    fn read() -> Result;
}
