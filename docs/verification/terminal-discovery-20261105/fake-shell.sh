#!/bin/sh
printf '%s %s\n' "$$" "$1" >> /tmp/gibbon-issue17/shell.log
/bin/sleep 1.2
printf '\nAGENT_GIBBON_PATH=/tmp/gibbon-issue17/bin:/usr/bin:/bin\n'
