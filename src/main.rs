fn main() {
    struct Structure(i32);
    impl std::fmt::Display for Structure {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
    println!("This struct `{}` prints just the number now!", Structure(3));

    #[derive(Debug)]
    struct User(String);

    println!("{:?}", User("Muneeb"));
}
