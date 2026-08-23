import re

with open("src/ast.rs", "r") as f:
    text = f.read()

# We want to replace the hardcoded enums with Traits
traits = """\
pub enum FieldType {
    String,
    Int,
    Real,
    Date,
}

pub trait ParseField: Clone + std::fmt::Display + PartialEq {
    fn parse(name: &str) -> Option<Self>;
    fn field_type(&self) -> FieldType;
}

pub trait ParseState: Clone + std::fmt::Display + PartialEq {
    fn parse(name: &str) -> Option<Self>;
}

pub trait ParseSort: Clone + std::fmt::Display + PartialEq {
    fn parse(name: &str) -> Option<Self>;
}
"""

text = re.sub(r'pub enum Field \{.*?(?=\n/// How a text field is matched)', traits, text, flags=re.DOTALL)
text = re.sub(r'impl Field \{.*?(?=\n/// How a text field is matched)', '', text, flags=re.DOTALL)
text = re.sub(r'pub enum State \{.*?(?=\n/// A `sort:` spec)', '', text, flags=re.DOTALL)
text = re.sub(r'impl State \{.*?(?=\n/// A `sort:` spec)', '', text, flags=re.DOTALL)
text = re.sub(r'pub enum SortKey \{.*?(?=\n/// The predicate AST)', '', text, flags=re.DOTALL)
text = re.sub(r'impl SortKey \{.*?(?=\n/// The predicate AST)', '', text, flags=re.DOTALL)
text = re.sub(r'/// A boolean state predicate.*?$', '', text, flags=re.MULTILINE)

# Make SortSpec generic
text = text.replace("pub struct SortSpec {", "pub struct SortSpec<K> {")
text = text.replace("pub key: SortKey,", "pub key: K,")

# Make Expr generic
text = text.replace("pub enum Expr {", "pub enum Expr<F, S> {")
text = text.replace("field: Field,", "field: F,")
text = text.replace("State(State),", "State(S),")
text = text.replace("Box<Expr>,", "Box<Expr<F, S>>,")
text = text.replace("Vec<Expr>,", "Vec<Expr<F, S>>,")

text = text.replace("impl fmt::Display for Expr {", "impl<F: ParseField, S: ParseState> fmt::Display for Expr<F, S> {")
text = text.replace("fn write_field(f: &mut fmt::Formatter<'_>, field: Field,", "fn write_field<F: ParseField>(f: &mut fmt::Formatter<'_>, field: F,")
text = text.replace("fn write_joined(f: &mut fmt::Formatter<'_>, items: &[Expr],", "fn write_joined<F: ParseField, S: ParseState>(f: &mut fmt::Formatter<'_>, items: &[Expr<F, S>],")
text = text.replace("fn paren(expr: &Expr) -> String {", "fn paren<F: ParseField, S: ParseState>(expr: &Expr<F, S>) -> String {")
text = text.replace("field.as_str()", "field")
text = text.replace("state.as_str()", "state")

with open("src/ast.rs", "w") as f:
    f.write(text)
