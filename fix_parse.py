with open("src/parse.rs", "r") as f:
    text = f.read()
text = text.replace(
"""pub fn parse<F: ParseField, S: ParseState, K: ParseSort>(input: &str) -> ParseResult<F, S, K> {
    parse_with_resolver::<F, S, K, ()>(input, &())
}""",
"""pub fn parse<F: ParseField, S: ParseState, K: ParseSort>(input: &str) -> ParseResult<F, S, K> {
    parse_inner::<F, S, K, ()>(input, None, &[])
}""")
with open("src/parse.rs", "w") as f:
    f.write(text)
