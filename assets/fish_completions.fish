function __timetrack_range_suggestions
    set -l token (commandline -ct)
    echo -e 0\n1\nhour\nday\nweek\nmonth
    if string match -qr '^(-[a-z])?[0-9]+' -- $token
        set -l n (string match -r '[0-9]+' -- $token)
        test "$n" = 1 \
            && echo $n\n"$n"hour\n"$n"day\n"$n"week\n"$n"month \
            || echo $n\n"$n"hours\n"$n"days\n"$n"weeks\n"$n"months
    end
end
function __timetrack_activity_subcommands
    timetrack activity help | awk '
        /^\w/ { c = 0 }
        /^Commands:/ { c = 1; next }
        !/^$/ && c {
            com = $1
            gsub(/^\s*\w+\s*/, "", $0)
            printf "%s\t\'%s\'\n", com, $0
        }
    '
end
function __timetrack_subcommands
    timetrack help | awk '
        /^\w/ { c = 0 }
        /^Commands:/ { c = 1; next }
        !/^$/ && c {
            com = $1
            gsub(/^\s*\w+\s*/, "", $0)
            printf "%s\t\'%s\'\n", com, $0
        }
    '
end
function __timetrack_attendance_types
    timetrack list-attendance-types -m \
        | awk -F "\t" '{ print $1 "\t\'" $2 "\'" }'
end
function __timetrack_activities
    set parent (commandline -ct | string replace -r '[^/]+$' '')
    timetrack activity ls -m $parent | awk -v p="$parent" -F "\t" '
        { printf "%s", p }
        $2 { printf "%s\t\'%s\'\n", $1, $2; next }
        { print $1 }
    '
end

complete -c timetrack -f \
    -n __fish_use_subcommand \
    -a "(__timetrack_subcommands)"

# Subcommand start
complete -c timetrack -f \
    -n '__fish_seen_subcommand_from start' \
    -a "(__timetrack_activities)"
complete -c timetrack \
    -rfl attendance \
    -n '__fish_seen_subcommand_from start' \
    -a "(__timetrack_attendance_types)"
complete -c timetrack \
    -rfs a \
    -n '__fish_seen_subcommand_from start' \
    -a "(__timetrack_attendance_types)"
complete -c timetrack \
    -rfl description \
    -n '__fish_seen_subcommand_from start'
complete -c timetrack \
    -rfs d \
    -n '__fish_seen_subcommand_from start'
complete -c timetrack \
    -fl help \
    -n '__fish_seen_subcommand_from start'
complete -c timetrack \
    -fs h \
    -n '__fish_seen_subcommand_from start'
complete -c timetrack -fl verbose \
    -n '__fish_seen_subcommand_from start'
complete -c timetrack -fs v \
    -n '__fish_seen_subcommand_from start'

# Subcommand show
complete -c timetrack -f \
    -n '__fish_seen_subcommand_from show' \
    -n 'not __fish_seen_subcommand_from entries collapsed attendance time' \
    -a "entries collapsed attendance time"
complete -c timetrack \
    -rfs l \
    -n '__fish_seen_subcommand_from show' \
    -a "(__timetrack_range_suggestions)"
complete -c timetrack \
    -rfl last \
    -n '__fish_seen_subcommand_from show' \
    -a "(__timetrack_range_suggestions)"
complete -c timetrack \
    -fl machine-readable \
    -n '__fish_seen_subcommand_from show'
complete -c timetrack \
    -fs m \
    -n '__fish_seen_subcommand_from show'
complete -c timetrack \
    -fl help \
    -n '__fish_seen_subcommand_from show'
complete -c timetrack \
    -fs h \
    -n '__fish_seen_subcommand_from show'

# Subcommand activity
complete -c timetrack -f \
    -n '__fish_seen_subcommand_from activity' \
    -n 'not __fish_seen_subcommand_from set rm mv ls help' \
    -a"(__timetrack_activity_subcommands)"

# Subcommand activity ls
complete -c timetrack -f \
    -n '__fish_seen_subcommand_from activity' \
    -n '__fish_seen_subcommand_from ls' \
    -a "(__timetrack_activities)"
complete -c timetrack \
    -fl recursive \
    -n '__fish_seen_subcommand_from activity' \
    -n '__fish_seen_subcommand_from ls'
complete -c timetrack \
    -fs r \
    -n '__fish_seen_subcommand_from activity' \
    -n '__fish_seen_subcommand_from ls'
complete -c timetrack \
    -fl machine-readable \
    -n '__fish_seen_subcommand_from activity' \
    -n '__fish_seen_subcommand_from ls'
complete -c timetrack \
    -fs m \
    -n '__fish_seen_subcommand_from activity' \
    -n '__fish_seen_subcommand_from ls'
complete -c timetrack \
    -fl help \
    -n '__fish_seen_subcommand_from activity' \
    -n '__fish_seen_subcommand_from ls'
complete -c timetrack \
    -fs h \
    -n '__fish_seen_subcommand_from activity' \
    -n '__fish_seen_subcommand_from ls'
