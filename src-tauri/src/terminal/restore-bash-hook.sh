if [[ -z $(trap -p DEBUG) ]]; then
    _jajan_at_prompt=0;
    _jajan_before_prompt() { _jajan_at_prompt=0; };
    _jajan_after_prompt() { _jajan_at_prompt=1; };
    _jajan_debug() {
        if [[ "$1" != _jajan_before_prompt && "$_jajan_at_prompt" == 1 ]]; then
            _jajan_at_prompt=0;
            _jajan_record "$1";
        fi;
    };
    if (( BASH_VERSINFO[0] > 5 || (BASH_VERSINFO[0] == 5 && BASH_VERSINFO[1] >= 1) )); then
        PROMPT_COMMAND=(_jajan_before_prompt "${PROMPT_COMMAND[@]}" _jajan_after_prompt);
    else
        PROMPT_COMMAND="_jajan_before_prompt;"$'\n'"${PROMPT_COMMAND-}"$'\n'"_jajan_after_prompt";
    fi;
    trap '_jajan_debug "$BASH_COMMAND"' DEBUG;
fi
