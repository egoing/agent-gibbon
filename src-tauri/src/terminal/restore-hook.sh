_gibbon_record() (
    set -f;
    local line="$1" name;
    line="${line#"${line%%[![:space:]]*}"}";
    name="${line%%[[:space:]]*}";
    name="${name%\'}";
    name="${name#\'}";
    name="${name##*/}";
    case "$name" in ''|[!a-zA-Z]*|*[!a-zA-Z0-9._-]*) name='' ;; esac;
    case "$line" in
        'df -h'|'df -k'|'du -h'|'du -k'|'ps -A'|'ps -e') name="$line" ;;
    esac;
    printf '\033]777;gibbon-plan;%s;%s\007' 'GIBBON_TOKEN' "$name";
)
