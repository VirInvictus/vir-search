import re
with open("src/rank.rs", "r") as f: text = f.read()
text = text.replace("fn walk(expr: &Expr,", "fn walk<F, S>(expr: &Expr<F, S>,")
text = text.replace("pub fn collect_text_terms(expr: &Expr)", "pub fn collect_text_terms<F, S>(expr: &Expr<F, S>)")
with open("src/rank.rs", "w") as f: f.write(text)
