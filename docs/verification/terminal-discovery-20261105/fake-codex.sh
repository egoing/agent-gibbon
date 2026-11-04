#!/bin/sh
printf 'ready %s %s\n' "$$" "$PWD" >> /tmp/gibbon-issue17/cli.log
printf 'GIBBON17_FAKE_READY\r\n'
while IFS= read -r line; do
  printf 'input %s %s\n' "$$" "$line" >> /tmp/gibbon-issue17/cli.log
  printf 'GIBBON17_ECHO:%s\r\n' "$line"
done
