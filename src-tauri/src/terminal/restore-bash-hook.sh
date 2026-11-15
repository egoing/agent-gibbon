if [[ -z $(trap -p DEBUG) ]]; then
    _gibbon_at_prompt=0;
    _gibbon_before_prompt() { _gibbon_at_prompt=0; };
    _gibbon_after_prompt() { _gibbon_at_prompt=1; };
    _gibbon_debug() {
        if [[ "$1" != _gibbon_before_prompt && "$_gibbon_at_prompt" == 1 ]]; then
            _gibbon_at_prompt=0;
            _gibbon_record "$1";
        fi;
    };
    if (( BASH_VERSINFO[0] > 5 || (BASH_VERSINFO[0] == 5 && BASH_VERSINFO[1] >= 1) )); then
        PROMPT_COMMAND=(_gibbon_before_prompt "${PROMPT_COMMAND[@]}" _gibbon_after_prompt);
    else
        PROMPT_COMMAND="_gibbon_before_prompt;"$'\n'"${PROMPT_COMMAND-}"$'\n'"_gibbon_after_prompt";
    fi;
    trap '_gibbon_debug "$BASH_COMMAND"' DEBUG;
fi
