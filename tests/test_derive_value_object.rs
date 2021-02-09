#[macro_use]
extern crate derive_value_object;

#[test]
fn test_derive_value_object() {
    #[derive(Debug, ValueObject)]
    #[value_object(load_fn="Value::new", error_type="String")]
    pub struct Value(String);

    impl Value {
        fn new(value: String) -> Result<Value, String> {
            return Ok(Value(value));
        }
    }
}