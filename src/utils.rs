pub fn safe_string_from_slice(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|c| format!("{:?}", *c as char).replace("'", ""))
        .collect::<String>()
}
