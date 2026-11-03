use potoo_gportugol_interpreter::{InputPrompt, RuntimeIo, execute};

#[derive(Default)]
struct Io(String);

impl RuntimeIo for Io {
    fn write(&mut self, text: &str) {
        self.0.push_str(text);
    }
    fn read_line(&mut self, _: &InputPrompt) -> Result<String, String> {
        Err("unexpected input".to_owned())
    }
}

#[test]
fn integer_operations_keep_gpt_32_bit_results() {
    let mut io = Io::default();
    execute("algoritmo limites; início imprima(-2147483648 / -1); imprima(-2147483648 % -1); imprima(2147483647 + 1); imprima(-2147483648 - 1); imprima(65536 * 65536); imprima(-(-2147483648)); fim", &mut io).unwrap();
    assert_eq!(
        io.0,
        "-2147483648\n0\n-2147483648\n2147483647\n0\n-2147483648\n"
    );
}

#[test]
fn integer_storage_and_bitwise_operations_use_32_bits() {
    let mut io = Io::default();
    execute("algoritmo conversoes; variáveis n: inteiro; r: real; fim-variáveis início n := 4294967297; imprima(n); imprima(~0xFFFFFFFF); imprima(+2147483648); r := 2147483648; imprima(r); fim", &mut io).unwrap();
    assert_eq!(io.0, "1\n0\n-2147483648\n2147483648.00\n");
}
