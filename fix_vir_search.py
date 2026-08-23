import re

with open("src/ast.rs", "r") as f:
    text = f.read()

text = text.replace("Today, Yesterday, ThisWeek, ThisMonth, ThisYear,", "Today, Yesterday, Tomorrow, ThisWeek, LastWeek, NextWeek, ThisMonth, ThisYear,")
text = text.replace('Self::Yesterday => write!(f, "yesterday"),', 'Self::Yesterday => write!(f, "yesterday"),\n            Self::Tomorrow => write!(f, "tomorrow"),')
text = text.replace('Self::ThisWeek => write!(f, "thisweek"),', 'Self::ThisWeek => write!(f, "thisweek"),\n            Self::LastWeek => write!(f, "lastweek"),\n            Self::NextWeek => write!(f, "nextweek"),')

with open("src/ast.rs", "w") as f:
    f.write(text)

