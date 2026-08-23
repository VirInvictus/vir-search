import re

with open("src/dates.rs", "r") as f:
    text = f.read()

# Replace resolve logic
text = text.replace("DateSpec::Yesterday => (", "DateSpec::Yesterday => (\n            today.pred_opt().unwrap().and_time(NaiveTime::MIN).and_utc().timestamp(),\n            today.and_time(NaiveTime::MIN).and_utc().timestamp(),\n        ),\n        DateSpec::Tomorrow => (")

text = text.replace("DateSpec::ThisWeek => {", "DateSpec::LastWeek => {\n            let w = today.week(Weekday::Mon);\n            let first = w.first_day() - chrono::Days::new(7);\n            let last = w.last_day() - chrono::Days::new(7);\n            (\n                first.and_time(NaiveTime::MIN).and_utc().timestamp(),\n                last.succ_opt().unwrap().and_time(NaiveTime::MIN).and_utc().timestamp(),\n            )\n        }\n        DateSpec::NextWeek => {\n            let w = today.week(Weekday::Mon);\n            let first = w.first_day() + chrono::Days::new(7);\n            let last = w.last_day() + chrono::Days::new(7);\n            (\n                first.and_time(NaiveTime::MIN).and_utc().timestamp(),\n                last.succ_opt().unwrap().and_time(NaiveTime::MIN).and_utc().timestamp(),\n            )\n        }\n        DateSpec::ThisWeek => {")

with open("src/dates.rs", "w") as f:
    f.write(text)
