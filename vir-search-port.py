import os, sys

def read(path):
    with open(path, "r") as f:
        return f.read()

def write(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(content)

os.makedirs("/home/bdkl/.gitrepos/vir-search/src", exist_ok=True)
write("/home/bdkl/.gitrepos/vir-search/src/lib.rs", """\
pub mod ast;
pub mod fold;
pub mod dates;
pub mod lex;
pub mod parse;
""")

print("Init successful.")
