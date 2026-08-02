#!/usr/bin/env python3
import re
import pathlib
import sys

log_path = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path('target/test-results/cargo-test.log')
out_path = pathlib.Path(sys.argv[2]) if len(sys.argv) > 2 else pathlib.Path('target/test-results/junit.xml')

log = log_path.read_text()
matches = re.findall(r'^test (.+?) \.\.\. (ok|FAIL|ignored|bench:)', log, flags=re.M)
count = len(matches)
failed = sum(1 for _, status in matches if status == 'FAIL')
ignored = sum(1 for _, status in matches if status == 'ignored')

xml = f'''<?xml version="1.0" encoding="UTF-8"?>
<testsuites tests="{count}" failures="{failed}" errors="0" skipped="{ignored}">
  <testsuite name="cargo test" tests="{count}" failures="{failed}" errors="0" skipped="{ignored}">
'''

for name, status in matches:
    normalized_name = re.sub(r'\s+', ' ', name.strip())
    escaped = normalized_name.replace('&', '&amp;').replace('<', '&lt;').replace('>', '&gt;').replace('"', '&quot;')
    if status == 'FAIL':
        xml += f'    <testcase classname="cargo" name="{escaped}"><failure message="failed"/></testcase>\n'
    elif status == 'ignored':
        xml += f'    <testcase classname="cargo" name="{escaped}"><skipped/></testcase>\n'
    else:
        xml += f'    <testcase classname="cargo" name="{escaped}"/>\n'

xml += '</testsuite>\n</testsuites>\n'
out_path.parent.mkdir(parents=True, exist_ok=True)
out_path.write_text(xml)
print(f'Wrote {out_path}')
